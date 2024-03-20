use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

static FEET_PER_METER: f64 = 3.28084;
static METERS_PER_MILE: f32 = 1609.344;
static ALTITUDE_THRESHOLD: f64 = 1.0;

/// Root node of the TCX document
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TrainingCenterDatabase {
    #[serde(rename = "Activities")]
    pub activities: Activities,
}

/// Contains a list of activities within this file
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Activities {
    #[serde(rename = "Activity")]
    pub activities: Vec<Activity>,
}

/// An individual activity, containing high level information
/// about the activity as well as all specific data points.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Activity {
    #[serde(rename = "Sport")]
    pub sport: String,

    /// The id for the activity, often the UTC timestamp of the activity start time.
    #[serde(rename = "Id")]
    pub id: String, // TODO: Is it guaranteed this is a timestamp? Could use DateTime<Utc> here.

    #[serde(rename = "Lap")]
    pub laps: Vec<Lap>,

    #[serde(rename = "Creator")]
    pub creator: Creator,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Creator {
    /// Device name that created this activity.
    #[serde(rename = "Name")]
    name: String,
}

/// Specific data for each Lap of the activity
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Lap {
    #[serde(rename = "StartTime")]
    pub start_time: DateTime<Utc>,

    #[serde(rename = "TotalTimeSeconds")]
    pub seconds: f32,

    #[serde(rename = "Calories")]
    pub calories: usize,

    /// Distance travelled in meters.
    #[serde(rename = "DistanceMeters")]
    pub distance: f32,

    /// Average HR for this lap
    #[serde(rename = "AverageHeartRateBpm")]
    average_hr: Option<HRValue>,

    /// Maximum HR for this lap
    #[serde(rename = "MaximumHeartRateBpm")]
    maximum_hr: Option<HRValue>,

    #[serde(rename = "Track")]
    track: Track,

    #[serde(rename = "Extensions")]
    extensions: Vec<LapExtension>, // TODO: Other fields - Intensity, TriggerMethod, MaximumSpeed

    /// Fields not parsed but used to calculate altitude gain/loss across [TrackPoints]
    #[serde(default)]
    last_alt: f64,
    #[serde(default)]
    alt_gain_meters: f64,
    #[serde(default)]
    alt_loss_meters: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct HRValue {
    #[serde(rename = "$value")]
    value: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct LapExtension {
    #[serde(rename = "LX")]
    lx: LXExtension,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct LXExtension {
    // TODO: What is this unit of measurement? m/s?
    #[serde(rename = "AvgSpeed")]
    avg_speed: f64,

    /// Average cadence in steps per minute for this lap. This is the steps done by one foot,
    /// so doubling the number gives a more typical cadence measurement.
    #[serde(rename = "AvgRunCadence")]
    avg_cadence: Option<usize>,

    /// Max cadence in steps per minute for this lap. This is the steps done by one foot,
    /// so doubling the number gives a more typical cadence measurement.
    #[serde(rename = "MaxRunCadence")]
    max_cadence: Option<usize>,

    /// Average watts as estimated by the device for this lap.
    #[serde(rename = "AvgWatts")]
    avg_watts: Option<usize>,

    /// Max watts as estimated by the device for this lap.
    #[serde(rename = "MaxWatts")]
    max_watts: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Track {
    #[serde(rename = "Trackpoint")]
    track_points: Vec<TrackPoint>,
}

/// There is a trackpoint every second for this activity
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TrackPoint {
    #[serde(rename = "Time")]
    time: DateTime<Utc>,

    #[serde(rename = "HeartRateBpm")]
    hr: Option<HRValue>,

    /// Distance (in meters) travelled
    #[serde(rename = "DistanceMeters")]
    distance: f32,

    /// Current altitude (in meters)
    #[serde(rename = "AltitudeMeters")]
    altitude: Option<f64>,

    /// Current Lat/Long position
    #[serde(rename = "Position")]
    position: Option<Position>,

    #[serde(rename = "Extensions")]
    extensions: Vec<TrackpointExtension>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Position {
    /// Latitude: Positive number indicates north of equator, negative indicates south.
    #[serde(rename = "LatitudeDegrees")]
    lat: f64,

    /// Longitude: Positive number indicates east of the prime meridian, negative indicates west.
    #[serde(rename = "LongitudeDegrees")]
    long: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TrackpointExtension {
    #[serde(rename = "TPX")]
    tpx: TPXExtension,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TPXExtension {
    // TODO: What is this unit of measurement? m/s?
    #[serde(rename = "Speed")]
    speed: Option<f64>,

    /// Current cadence in steps per minute. This is the steps done by one foot,
    /// so doubling the number gives a more typical cadence measurement.
    #[serde(rename = "RunCadence")]
    cadence: usize,

    /// Current watts as estimated by the device.
    #[serde(rename = "Watts")]
    watts: Option<usize>,
}

/************* IMPLS **************/

impl TrainingCenterDatabase {
    pub fn get_activity(&self, idx: usize) -> Option<&Activity> {
        self.activities.activities.get(idx)
    }

    pub fn get_activity_mut(&mut self, idx: usize) -> Option<&mut Activity> {
        self.activities.activities.get_mut(idx)
    }
}

impl Activity {
    pub fn creator(&self) -> &str {
        self.creator.name.as_str()
    }

    pub fn lap_count(&self) -> usize {
        self.laps.len()
    }

    pub fn average_hr(&self) -> usize {
        if self.lap_count() == 0 {
            return 0;
        }

        let mut total_hr = 0;
        let mut total_divisor = 0;
        for lap in self.laps.iter() {
            total_hr += lap.total_hr();
            total_divisor += lap.total_measurements();
        }
        total_hr / total_divisor
    }

    /// Average pace in meters/s.
    fn average_pace_meters(&self) -> f32 {
        if self.lap_count() == 0 {
            return 0.0;
        }

        let mut total_time = 0.0;
        let mut total_distance = 0.0;
        for lap in self.laps.iter() {
            total_time += lap.seconds;
            total_distance += lap.distance;
        }

        total_distance / total_time
    }

    // Return average pace in miles/minute, formatted as a time "MM:SS"
    pub fn average_pace(&self) -> String {
        if self.lap_count() == 0 {
            return String::from("00:00 / mi");
        }

        let duration = self.average_pace_seconds();
        format!(
            "{:02}:{:02} / mi",
            duration.as_secs() / 60,
            duration.as_secs() % 60
        )
    }

    pub fn average_pace_seconds(&self) -> std::time::Duration {
        let seconds_per_mile = (METERS_PER_MILE / self.average_pace_meters()).round() as u64;
        std::time::Duration::new(seconds_per_mile, 0)
    }

    pub fn total_distance_meters(&self) -> f32 {
        self.laps.iter().map(|l| l.distance).sum()
    }

    pub fn total_distance_miles(&self) -> f32 {
        0.0006213712 * self.total_distance_meters()
    }

    /// Total elevation gain in feet.
    pub fn total_elevation_gain(&self) -> usize {
        let gain_meters = self
            .laps
            .iter()
            .map(|l| l.alt_gain_meters)
            .fold(0.0, |sum, v| sum + v);
        (gain_meters * FEET_PER_METER).round() as usize
    }

    /// Total elevation loss in feet.
    pub fn total_elevation_loss(&self) -> usize {
        let loss_meters = self
            .laps
            .iter()
            .map(|l| l.alt_loss_meters)
            .fold(0.0, |sum, v| sum + v);
        (loss_meters * FEET_PER_METER).round() as usize
    }

    pub fn calc_lap_elevations(&mut self) {
        self.laps.iter_mut().for_each(|l| l.calc_elevation());
    }

    pub fn average_cadence(&self) -> usize {
        let total_cadence: usize = self
            .laps
            .iter()
            .filter_map(|l| l.extensions.get(0))
            .filter_map(|ext| ext.lx.avg_cadence)
            .sum();
        (total_cadence / self.lap_count()) * 2
    }

    pub fn average_watts(&self) -> usize {
        let total_watts: usize = self
            .laps
            .iter()
            .filter_map(|l| l.extensions.get(0))
            .map(|ext| ext.lx.avg_watts.unwrap_or(0))
            .sum();
        total_watts / self.lap_count()
    }
}

impl Lap {
    /// The total amount of Trackpoint measurements
    /// this lap contains.
    fn total_measurements(&self) -> usize {
        self.track.track_points.len()
    }

    /// The total of all individual HR values, used to calculate
    /// the average HR across multiple laps.
    fn total_hr(&self) -> usize {
        self.track
            .track_points
            .iter()
            .filter_map(|tp| tp.hr.as_ref())
            .map(|hr| hr.value)
            .sum()
    }

    fn calc_elevation(&mut self) {
        self.last_alt = if let Some(tp) = self.track.track_points.first() {
            tp.altitude.unwrap_or(0.0)
        } else {
            0.0
        };

        for tp in self.track.track_points.iter() {
            if let Some(altitude) = tp.altitude {
                let alt_change = (altitude - self.last_alt).abs();
                if alt_change < ALTITUDE_THRESHOLD {
                    continue;
                }

                if altitude > self.last_alt {
                    self.alt_gain_meters += alt_change;
                } else {
                    self.alt_loss_meters += alt_change;
                }

                self.last_alt = altitude;
            }
        }
    }

    /*
    TODO: Average watts, average cadence
     */
}
