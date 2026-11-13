//! Great-circle geometry. All internal distances are nautical miles.

pub const EARTH_RADIUS_NM: f64 = 3440.065;
pub const NM_PER_MI: f64 = 0.868_976;
pub const NM_PER_KM: f64 = 0.539_957;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DistUnit {
    #[default]
    Nm,
    Mi,
    Km,
}

impl DistUnit {
    pub fn to_nm(self, v: f64) -> f64 {
        match self {
            DistUnit::Nm => v,
            DistUnit::Mi => v * NM_PER_MI,
            DistUnit::Km => v * NM_PER_KM,
        }
    }
    pub fn from_nm(self, nm: f64) -> f64 {
        match self {
            DistUnit::Nm => nm,
            DistUnit::Mi => nm / NM_PER_MI,
            DistUnit::Km => nm / NM_PER_KM,
        }
    }
}

/// Haversine distance in nautical miles.
pub fn distance_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dphi = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dphi / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_NM * a.sqrt().min(1.0).asin()
}

/// Initial bearing from point 1 to point 2, degrees 0..360.
pub fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

/// Smallest absolute angle between two headings, 0..180.
pub fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    if d > 180.0 { 360.0 - d } else { d }
}

/// Destination point given start, bearing and distance.
pub fn destination(lat: f64, lon: f64, bearing: f64, dist_nm: f64) -> (f64, f64) {
    let d = dist_nm / EARTH_RADIUS_NM;
    let (b, p1, l1) = (bearing.to_radians(), lat.to_radians(), lon.to_radians());
    let p2 = (p1.sin() * d.cos() + p1.cos() * d.sin() * b.cos()).asin();
    let l2 = l1 + (b.sin() * d.sin() * p1.cos()).atan2(d.cos() - p1.sin() * p2.sin());
    (p2.to_degrees(), ((l2.to_degrees() + 540.0) % 360.0) - 180.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boston_to_new_york() {
        // KBOS -> KJFK is ~162 nm
        let d = distance_nm(42.3643, -71.0052, 40.6398, -73.7789);
        assert!((d - 162.0).abs() < 3.0, "{d}");
    }
    #[test]
    fn zero_distance() {
        assert!(distance_nm(10.0, 20.0, 10.0, 20.0) < 1e-9);
    }
    #[test]
    fn antipodal_is_finite() {
        let d = distance_nm(0.0, 0.0, 0.0, 180.0);
        assert!((d - std::f64::consts::PI * EARTH_RADIUS_NM).abs() < 1.0);
    }
    #[test]
    fn dateline_crossing() {
        let d = distance_nm(0.0, 179.5, 0.0, -179.5);
        assert!((d - 60.0).abs() < 1.0, "{d}");
    }
    #[test]
    fn bearings() {
        assert!((bearing_deg(0.0, 0.0, 1.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((bearing_deg(0.0, 0.0, 0.0, 1.0) - 90.0).abs() < 1e-6);
        assert!((bearing_deg(0.0, 0.0, -1.0, 0.0) - 180.0).abs() < 1e-6);
        assert!((bearing_deg(0.0, 0.0, 0.0, -1.0) - 270.0).abs() < 1e-6);
    }
    #[test]
    fn angle_wrap() {
        assert_eq!(angle_diff(350.0, 10.0), 20.0);
        assert_eq!(angle_diff(10.0, 350.0), 20.0);
        assert_eq!(angle_diff(0.0, 180.0), 180.0);
    }
    #[test]
    fn destination_roundtrip() {
        let (la, lo) = destination(42.0, -71.0, 45.0, 25.0);
        assert!((distance_nm(42.0, -71.0, la, lo) - 25.0).abs() < 0.01);
        assert!((bearing_deg(42.0, -71.0, la, lo) - 45.0).abs() < 0.2);
    }
    #[test]
    fn units() {
        assert!((DistUnit::Mi.to_nm(10.0) - 8.69).abs() < 0.01);
        assert!((DistUnit::Km.from_nm(DistUnit::Km.to_nm(7.0)) - 7.0).abs() < 1e-9);
    }
}
