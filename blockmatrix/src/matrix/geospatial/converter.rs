// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! GPS to Matrix coordinate conversion
//!
//! Converts between geographic coordinates (latitude/longitude) and
//! Block-MATRIX coordinate system with configurable scale factors.

use crate::matrix::coordinate::MatrixCoordinate;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Earth's radius in kilometers (WGS84 standard)
const EARTH_RADIUS_KM: f64 = 6371.0;

/// Maximum valid latitude (90 degrees)
const MAX_LATITUDE: f64 = 90.0;

/// Maximum valid longitude (180 degrees)
const MAX_LONGITUDE: f64 = 180.0;

/// GPS conversion errors
#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum GpsError {
    /// Invalid latitude value
    #[error("Invalid latitude: {0}. Must be between -90 and 90")]
    InvalidLatitude(f64),

    /// Invalid longitude value
    #[error("Invalid longitude: {0}. Must be between -180 and 180")]
    InvalidLongitude(f64),

    /// Invalid elevation value
    #[error("Invalid elevation: {0}")]
    InvalidElevation(f64),

    /// Invalid scale factor
    #[error("Invalid scale factor: {0}. Must be positive")]
    InvalidScale(f64),

    /// Conversion overflow
    #[error("Coordinate conversion overflow")]
    ConversionOverflow,

    /// NaN or infinite value
    #[error("Invalid numeric value: {0}")]
    InvalidNumeric(String),
}

/// Scale resolution for GPS to matrix conversion
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ScaleResolution {
    /// 1 matrix unit = 0.1 km (100m resolution)
    Fine,
    /// 1 matrix unit = 1 km
    Standard,
    /// 1 matrix unit = 10 km
    Coarse,
    /// 1 matrix unit = 100 km
    Regional,
    /// Custom scale factor (units per km)
    Custom(f64),
}

impl ScaleResolution {
    /// Get units per kilometer for this resolution
    pub fn units_per_km(&self) -> f64 {
        match self {
            ScaleResolution::Fine => 10.0,
            ScaleResolution::Standard => 1.0,
            ScaleResolution::Coarse => 0.1,
            ScaleResolution::Regional => 0.01,
            ScaleResolution::Custom(scale) => *scale,
        }
    }
}

/// GPS coordinate representation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpsCoordinate {
    /// Latitude in degrees (-90 to 90)
    pub latitude: f64,
    /// Longitude in degrees (-180 to 180)
    pub longitude: f64,
    /// Elevation in meters (optional, defaults to 0)
    pub elevation: f64,
}

impl GpsCoordinate {
    /// Create a new GPS coordinate
    pub fn new(latitude: f64, longitude: f64, elevation: f64) -> Result<Self, GpsError> {
        if !latitude.is_finite() {
            return Err(GpsError::InvalidNumeric(format!("latitude: {}", latitude)));
        }
        if !longitude.is_finite() {
            return Err(GpsError::InvalidNumeric(format!("longitude: {}", longitude)));
        }
        if !elevation.is_finite() {
            return Err(GpsError::InvalidNumeric(format!("elevation: {}", elevation)));
        }

        if latitude < -MAX_LATITUDE || latitude > MAX_LATITUDE {
            return Err(GpsError::InvalidLatitude(latitude));
        }
        if longitude < -MAX_LONGITUDE || longitude > MAX_LONGITUDE {
            return Err(GpsError::InvalidLongitude(longitude));
        }

        Ok(Self {
            latitude,
            longitude,
            elevation,
        })
    }

    /// Create GPS coordinate at sea level
    pub fn at_sea_level(latitude: f64, longitude: f64) -> Result<Self, GpsError> {
        Self::new(latitude, longitude, 0.0)
    }
}

/// GPS to Matrix coordinate converter
#[derive(Debug, Clone)]
pub struct GpsConverter {
    /// Scale resolution for conversion
    resolution: ScaleResolution,
    /// Origin GPS coordinate (maps to matrix 0,0,0)
    origin: GpsCoordinate,
}

impl GpsConverter {
    /// Create a new converter with given resolution
    pub fn new(resolution: ScaleResolution) -> Self {
        Self {
            resolution,
            // Default origin at 0,0 (Equator/Prime Meridian intersection)
            origin: GpsCoordinate {
                latitude: 0.0,
                longitude: 0.0,
                elevation: 0.0,
            },
        }
    }

    /// Create converter with custom origin
    pub fn with_origin(resolution: ScaleResolution, origin: GpsCoordinate) -> Self {
        Self { resolution, origin }
    }

    /// Convert GPS coordinate to matrix coordinate
    pub fn gps_to_matrix(&self, gps: &GpsCoordinate) -> Result<MatrixCoordinate, GpsError> {
        let scale = self.resolution.units_per_km();

        // Convert degrees to radians
        let lat_rad = gps.latitude * PI / 180.0;
        let lon_rad = gps.longitude * PI / 180.0;
        let origin_lat_rad = self.origin.latitude * PI / 180.0;
        let origin_lon_rad = self.origin.longitude * PI / 180.0;

        // Calculate relative differences
        let lat_diff = lat_rad - origin_lat_rad;
        let lon_diff = lon_rad - origin_lon_rad;

        // Convert to kilometers using spherical approximation
        // x = longitude difference * Earth radius * cos(average latitude)
        let avg_lat = (lat_rad + origin_lat_rad) / 2.0;
        let x_km = lon_diff * EARTH_RADIUS_KM * avg_lat.cos();
        let y_km = lat_diff * EARTH_RADIUS_KM;

        // Convert to matrix units
        let x = (x_km * scale).round() as i64;
        let y = (y_km * scale).round() as i64;

        // Z coordinate represents elevation or network layer
        // Convert elevation from meters to kilometers then to matrix units
        let z_km = (gps.elevation - self.origin.elevation) / 1000.0;
        let z = (z_km * scale).round() as i64;

        MatrixCoordinate::new(x, y, z)
            .map_err(|_| GpsError::ConversionOverflow)
    }

    /// Convert matrix coordinate back to GPS
    pub fn matrix_to_gps(&self, matrix: &MatrixCoordinate) -> Result<GpsCoordinate, GpsError> {
        let scale = self.resolution.units_per_km();

        // Convert matrix units to kilometers
        let x_km = matrix.x as f64 / scale;
        let y_km = matrix.y as f64 / scale;
        let z_km = matrix.z as f64 / scale;

        // Convert y (north-south) to latitude difference
        let lat_diff_rad = y_km / EARTH_RADIUS_KM;
        let latitude_rad = self.origin.latitude * PI / 180.0 + lat_diff_rad;

        // Convert x (east-west) to longitude difference
        // Need to account for latitude in the conversion
        let avg_lat_rad = (latitude_rad + self.origin.latitude * PI / 180.0) / 2.0;
        let lon_diff_rad = x_km / (EARTH_RADIUS_KM * avg_lat_rad.cos());
        let longitude_rad = self.origin.longitude * PI / 180.0 + lon_diff_rad;

        // Convert radians back to degrees
        let latitude = latitude_rad * 180.0 / PI;
        let longitude = longitude_rad * 180.0 / PI;

        // Handle wraparound for longitude
        let longitude = if longitude > 180.0 {
            longitude - 360.0
        } else if longitude < -180.0 {
            longitude + 360.0
        } else {
            longitude
        };

        // Convert elevation back to meters
        let elevation = self.origin.elevation + z_km * 1000.0;

        GpsCoordinate::new(latitude, longitude, elevation)
    }

    /// Get the resolution of this converter
    pub fn resolution(&self) -> ScaleResolution {
        self.resolution
    }

    /// Calculate GPS distance between two points in kilometers
    pub fn gps_distance_km(a: &GpsCoordinate, b: &GpsCoordinate) -> f64 {
        // Haversine formula for great circle distance
        let lat1_rad = a.latitude * PI / 180.0;
        let lat2_rad = b.latitude * PI / 180.0;
        let lon1_rad = a.longitude * PI / 180.0;
        let lon2_rad = b.longitude * PI / 180.0;

        let dlat = lat2_rad - lat1_rad;
        let dlon = lon2_rad - lon1_rad;

        let a = (dlat / 2.0).sin().powi(2) +
                lat1_rad.cos() * lat2_rad.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_KM * c
    }
}

impl Default for GpsConverter {
    fn default() -> Self {
        Self::new(ScaleResolution::Standard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gps_coordinate_validation() {
        // Valid coordinates
        assert!(GpsCoordinate::new(45.0, -122.0, 100.0).is_ok());
        assert!(GpsCoordinate::new(-90.0, 180.0, 0.0).is_ok());
        assert!(GpsCoordinate::new(90.0, -180.0, -100.0).is_ok());

        // Invalid latitude
        assert!(matches!(
            GpsCoordinate::new(91.0, 0.0, 0.0),
            Err(GpsError::InvalidLatitude(_))
        ));
        assert!(matches!(
            GpsCoordinate::new(-91.0, 0.0, 0.0),
            Err(GpsError::InvalidLatitude(_))
        ));

        // Invalid longitude
        assert!(matches!(
            GpsCoordinate::new(0.0, 181.0, 0.0),
            Err(GpsError::InvalidLongitude(_))
        ));
        assert!(matches!(
            GpsCoordinate::new(0.0, -181.0, 0.0),
            Err(GpsError::InvalidLongitude(_))
        ));

        // NaN values
        assert!(matches!(
            GpsCoordinate::new(f64::NAN, 0.0, 0.0),
            Err(GpsError::InvalidNumeric(_))
        ));
        assert!(matches!(
            GpsCoordinate::new(0.0, f64::INFINITY, 0.0),
            Err(GpsError::InvalidNumeric(_))
        ));
    }

    #[test]
    fn test_scale_resolution() {
        assert_eq!(ScaleResolution::Fine.units_per_km(), 10.0);
        assert_eq!(ScaleResolution::Standard.units_per_km(), 1.0);
        assert_eq!(ScaleResolution::Coarse.units_per_km(), 0.1);
        assert_eq!(ScaleResolution::Regional.units_per_km(), 0.01);
        assert_eq!(ScaleResolution::Custom(5.0).units_per_km(), 5.0);
    }

    #[test]
    fn test_equator_conversion() {
        let converter = GpsConverter::new(ScaleResolution::Standard);

        // Points on the equator
        let gps = GpsCoordinate::at_sea_level(0.0, 1.0).unwrap(); // ~111km east
        let matrix = converter.gps_to_matrix(&gps).unwrap();

        // At equator, 1 degree longitude ≈ 111km
        assert!((matrix.x as f64 - 111.0).abs() < 1.0);
        assert_eq!(matrix.y, 0);
        assert_eq!(matrix.z, 0);
    }

    #[test]
    fn test_latitude_conversion() {
        let converter = GpsConverter::new(ScaleResolution::Standard);

        // 1 degree latitude ≈ 111km everywhere
        let gps = GpsCoordinate::at_sea_level(1.0, 0.0).unwrap();
        let matrix = converter.gps_to_matrix(&gps).unwrap();

        assert_eq!(matrix.x, 0);
        assert!((matrix.y as f64 - 111.0).abs() < 1.0);
        assert_eq!(matrix.z, 0);
    }

    #[test]
    fn test_round_trip_conversion() {
        let converter = GpsConverter::new(ScaleResolution::Fine);

        // Test various coordinates
        let test_coords = vec![
            GpsCoordinate::new(45.5, -122.6, 100.0).unwrap(), // Portland
            GpsCoordinate::new(-33.9, 151.2, 50.0).unwrap(),  // Sydney
            GpsCoordinate::new(51.5, -0.1, 10.0).unwrap(),    // London
            GpsCoordinate::new(35.7, 139.7, 30.0).unwrap(),   // Tokyo
        ];

        for original in test_coords {
            let matrix = converter.gps_to_matrix(&original).unwrap();
            let recovered = converter.matrix_to_gps(&matrix).unwrap();

            // Check accuracy (should be within ~0.1 degrees with Fine resolution)
            assert!((recovered.latitude - original.latitude).abs() < 0.1);
            assert!((recovered.longitude - original.longitude).abs() < 0.1);
            // Elevation accuracy within 100m
            assert!((recovered.elevation - original.elevation).abs() < 100.0);
        }
    }

    #[test]
    fn test_pole_conversion() {
        let converter = GpsConverter::new(ScaleResolution::Standard);

        // North pole
        let north_pole = GpsCoordinate::at_sea_level(90.0, 0.0).unwrap();
        let matrix = converter.gps_to_matrix(&north_pole).unwrap();
        assert_eq!(matrix.x, 0); // Longitude is undefined at poles
        assert!((matrix.y as f64 - 10007.0).abs() < 10.0); // ~10007km from equator

        // South pole
        let south_pole = GpsCoordinate::at_sea_level(-90.0, 0.0).unwrap();
        let matrix2 = converter.gps_to_matrix(&south_pole).unwrap();
        assert_eq!(matrix2.x, 0);
        assert!((matrix2.y as f64 + 10007.0).abs() < 10.0);
    }

    #[test]
    fn test_date_line_crossing() {
        let converter = GpsConverter::new(ScaleResolution::Standard);

        // Points near the International Date Line
        let west = GpsCoordinate::at_sea_level(0.0, 179.0).unwrap();
        let east = GpsCoordinate::at_sea_level(0.0, -179.0).unwrap();

        let matrix_west = converter.gps_to_matrix(&west).unwrap();
        let matrix_east = converter.gps_to_matrix(&east).unwrap();

        // They should be close in matrix space (2 degrees apart)
        let distance = ((matrix_west.x - matrix_east.x).abs() as f64);
        assert!(distance < 250.0); // ~222km at equator
    }

    #[test]
    fn test_elevation_conversion() {
        let converter = GpsConverter::new(ScaleResolution::Standard);

        // Mount Everest height
        let everest = GpsCoordinate::new(27.9881, 86.9250, 8848.0).unwrap();
        let matrix = converter.gps_to_matrix(&everest).unwrap();

        // Z should represent ~8.8km elevation
        assert!((matrix.z as f64 - 8.8).abs() < 1.0);
    }

    #[test]
    fn test_custom_origin() {
        // Set origin at New York City
        let nyc = GpsCoordinate::new(40.7128, -74.0060, 10.0).unwrap();
        let converter = GpsConverter::with_origin(ScaleResolution::Standard, nyc);

        // NYC should map to (0,0,0)
        let matrix = converter.gps_to_matrix(&nyc).unwrap();
        assert_eq!(matrix.x, 0);
        assert_eq!(matrix.y, 0);
        assert_eq!(matrix.z, 0);

        // Philadelphia (~150km south)
        let philly = GpsCoordinate::new(39.9526, -75.1652, 10.0).unwrap();
        let matrix2 = converter.gps_to_matrix(&philly).unwrap();
        assert!(matrix2.y < -80); // South is negative Y
        assert!((matrix2.x as f64 + 80.0).abs() < 40.0); // West
    }

    #[test]
    fn test_gps_distance() {
        // New York to Los Angeles (~3935 km)
        let nyc = GpsCoordinate::at_sea_level(40.7128, -74.0060).unwrap();
        let la = GpsCoordinate::at_sea_level(34.0522, -118.2437).unwrap();

        let distance = GpsConverter::gps_distance_km(&nyc, &la);
        assert!((distance - 3935.0).abs() < 50.0); // Within 50km accuracy

        // London to Paris (~344 km)
        let london = GpsCoordinate::at_sea_level(51.5074, -0.1278).unwrap();
        let paris = GpsCoordinate::at_sea_level(48.8566, 2.3522).unwrap();

        let distance2 = GpsConverter::gps_distance_km(&london, &paris);
        assert!((distance2 - 344.0).abs() < 10.0);
    }
}