use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use plotters::prelude::*;
use serde_xml_rs::from_str;
use stats::ActivityStats;
use tracing::{info, instrument};

pub use crate::tcx::*;
pub mod stats;
pub mod tcx;

#[instrument]
pub async fn parse_file(filename: &Path) -> Result<TrainingCenterDatabase> {
    info!("Parsing: {}", filename.display());
    let file_data = tokio::fs::read_to_string(filename).await?;
    let tcb = from_str(file_data.as_str())?;
    info!("Successfully parsed: {}", filename.display());
    Ok(tcb)
}

fn all_tcx_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(anyhow!("Directory {} is not a folder.", dir.display()));
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            paths.extend(all_tcx_paths(&path)?);
        } else if path.extension().is_some_and(|e| e == "tcx") {
            paths.push(path);
        }
    }
    Ok(paths)
}

#[cfg(not(feature = "slow"))]
#[instrument]
pub async fn parse_folder(folder: &Path) -> Result<Vec<TrainingCenterDatabase>> {
    let paths: Vec<PathBuf> = all_tcx_paths(folder)?;

    let mut join_handles = vec![];
    for path in paths {
        join_handles.push(tokio::spawn(async move {
            parse_file(&path)
                .await
                .expect(format!("Error parsing {}", path.display()).as_str())
        }));
    }
    let parsed_results = futures::future::join_all(join_handles)
        .await
        .into_iter()
        .map(|tcx| tcx.unwrap())
        .collect();
    Ok(parsed_results)
}

#[cfg(feature = "slow")]
#[instrument]
pub async fn parse_folder(folder: &Path) -> Result<Vec<TrainingCenterDatabase>> {
    let paths = all_tcx_paths(folder)?;

    let mut parsed_results = vec![];
    for path in paths {
        parsed_results.push(parse_file(&path).await?);
    }
    Ok(parsed_results)
}

pub async fn display_folder_stats(
    folder: &Path,
    output: &Path,
    chart_filename: String,
) -> Result<()> {
    let mut parsed_results = parse_folder(folder).await?;

    let mut activities: Vec<_> = parsed_results
        .iter_mut()
        .filter_map(|tcb| {
            let activity = tcb.get_activity_mut(0)?;
            activity.calc_lap_elevations();
            // Return an immutable activity after mutating.
            Some(&*activity)
        })
        .collect();

    activities.sort_by(|a1, a2| a1.id.cmp(&a2.id));

    let mut activity_stats = vec![];
    let mut output_file = File::create(output)?;
    for activity in activities {
        let activity_stat = ActivityStats::from(activity);
        activity_stat.write_to(&mut output_file)?;
        activity_stats.push(activity_stat);
    }

    info!("Processed {} activities", activity_stats.len());
    chart(chart_filename, activity_stats)?;

    Ok(())
}

fn chart(chart_filename: String, activity_stats: Vec<ActivityStats>) -> Result<()> {
    let x_range = 0usize..activity_stats.len();

    let pace = activity_stats
        .iter()
        .enumerate()
        .map(|(i, stats)| (i, stats.average_pace_seconds.as_secs()))
        .collect::<Vec<(usize, u64)>>();

    let hr = activity_stats
        .iter()
        .enumerate()
        .map(|(i, stats)| (i, stats.average_hr))
        .collect::<Vec<(usize, usize)>>();

    let root = BitMapBackend::new(chart_filename.as_str(), (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .right_y_label_area_size(40)
        .margin(5)
        .caption(
            "Avg pace vs. Avg heart rate",
            ("sans-serif", 50.0).into_font(),
        )
        .build_cartesian_2d(x_range.clone(), 420u64..550u64)?
        .set_secondary_coord(x_range, 115usize..180usize);

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_desc("Activity number")
        .y_desc("Pace (seconds per mile)")
        .draw()?;

    chart
        .configure_secondary_axes()
        .y_desc("Heart rate")
        .draw()?;

    chart
        .draw_series(LineSeries::new(pace, &BLUE))?
        .label("Seconds per mile")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart
        .draw_secondary_series(LineSeries::new(hr, &RED))?
        .label("Heart rate")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .background_style(RGBColor(128, 128, 128))
        .draw()?;

    root.present().expect("Unable to write result to file");
    info!("Chart has been saved to {}", chart_filename);

    Ok(())
}
