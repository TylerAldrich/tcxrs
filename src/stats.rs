use anyhow::Result;
use std::{fs::File, io::Write, time::Duration};

use crate::Activity;

pub struct ActivityStats {
    date: String,
    laps: usize,
    distance_mi: f32,
    distance_km: f32,
    pub average_hr: usize,
    average_pace: String,
    pub average_pace_seconds: Duration,
    average_watts: usize,
    average_cadence: usize,
    elevation_gain: usize,
    elevation_loss: usize,
}

impl ActivityStats {
    pub fn new(activity: &Activity) -> Self {
        ActivityStats {
            date: activity.id.clone(),
            laps: activity.lap_count(),
            distance_mi: activity.total_distance_miles(),
            distance_km: activity.total_distance_meters() / 1000.0,
            average_hr: activity.average_hr(),
            average_pace: activity.average_pace(),
            average_pace_seconds: activity.average_pace_seconds(),
            average_watts: activity.average_watts(),
            average_cadence: activity.average_cadence(),
            elevation_gain: activity.total_elevation_gain(),
            elevation_loss: activity.total_elevation_loss(),
        }
    }
}

impl From<&Activity> for ActivityStats {
    fn from(activity: &Activity) -> ActivityStats {
        ActivityStats::new(&activity)
    }
}

impl ActivityStats {
    pub fn stats(&self) -> Vec<String> {
        let mut stats = vec![];
        stats.push(format!("=== {} ===", self.date));
        stats.push(format!("  Total laps: {}", self.laps));
        stats.push(format!(
            "  Distance: {:.2}mi / {:.2}km",
            self.distance_mi, self.distance_km
        ));
        stats.push(format!("  Average HR: {}", self.average_hr));
        stats.push(format!("  Average Pace: {}", self.average_pace));

        stats.push(format!("  Average Power: {}W", self.average_watts));
        stats.push(format!(
            "  Average Cadence: {} steps/min",
            self.average_cadence
        ));

        stats.push(format!("  Elevation Gain: {}", self.elevation_gain));
        stats.push(format!("  Elevation Loss: {}", self.elevation_loss));
        stats.push(String::from("================================\n\n"));
        stats
    }
    pub fn display(&self) {
        for val in self.stats() {
            println!("{}", val);
        }
    }

    pub fn write_to(&self, output_file: &mut File) -> Result<()> {
        output_file.write_all(self.stats().join("\n").as_bytes())?;
        Ok(())
    }
}
