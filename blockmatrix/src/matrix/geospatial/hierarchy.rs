// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Geographic hierarchy and zone management
//!
//! Provides hierarchical geographic zones (country/region/city)
//! for organizing nodes by real-world locations.

use crate::matrix::geospatial::converter::{GpsCoordinate, GpsError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Geographic hierarchy levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum GeographicLevel {
    /// Global level (entire world)
    Global,
    /// Continental level (e.g., North America, Europe)
    Continent,
    /// Country level (e.g., United States, Germany)
    Country,
    /// Region/State level (e.g., California, Bavaria)
    Region,
    /// City level (e.g., San Francisco, Munich)
    City,
    /// Local level (neighborhood, district)
    Local,
}

impl GeographicLevel {
    /// Get the parent level (one level up in hierarchy)
    pub fn parent_level(&self) -> Option<GeographicLevel> {
        match self {
            GeographicLevel::Global => None,
            GeographicLevel::Continent => Some(GeographicLevel::Global),
            GeographicLevel::Country => Some(GeographicLevel::Continent),
            GeographicLevel::Region => Some(GeographicLevel::Country),
            GeographicLevel::City => Some(GeographicLevel::Region),
            GeographicLevel::Local => Some(GeographicLevel::City),
        }
    }

    /// Get the child level (one level down in hierarchy)
    pub fn child_level(&self) -> Option<GeographicLevel> {
        match self {
            GeographicLevel::Global => Some(GeographicLevel::Continent),
            GeographicLevel::Continent => Some(GeographicLevel::Country),
            GeographicLevel::Country => Some(GeographicLevel::Region),
            GeographicLevel::Region => Some(GeographicLevel::City),
            GeographicLevel::City => Some(GeographicLevel::Local),
            GeographicLevel::Local => None,
        }
    }
}

/// Geographic bounds as a GPS rectangle
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeographicBounds {
    /// Minimum latitude (southern boundary)
    pub min_latitude: f64,
    /// Maximum latitude (northern boundary)
    pub max_latitude: f64,
    /// Minimum longitude (western boundary)
    pub min_longitude: f64,
    /// Maximum longitude (eastern boundary)
    pub max_longitude: f64,
}

impl GeographicBounds {
    /// Create new bounds
    pub fn new(
        min_lat: f64,
        max_lat: f64,
        min_lon: f64,
        max_lon: f64,
    ) -> Result<Self, GpsError> {
        if min_lat > max_lat {
            return Err(GpsError::InvalidLatitude(min_lat));
        }
        if min_lon > max_lon && !Self::crosses_date_line(min_lon, max_lon) {
            return Err(GpsError::InvalidLongitude(min_lon));
        }

        Ok(Self {
            min_latitude: min_lat,
            max_latitude: max_lat,
            min_longitude: min_lon,
            max_longitude: max_lon,
        })
    }

    /// Check if bounds cross the International Date Line
    fn crosses_date_line(min_lon: f64, max_lon: f64) -> bool {
        min_lon > max_lon // e.g., min=170, max=-170
    }

    /// Check if a GPS coordinate is within these bounds
    pub fn contains(&self, coord: &GpsCoordinate) -> bool {
        // Latitude check is straightforward
        if coord.latitude < self.min_latitude || coord.latitude > self.max_latitude {
            return false;
        }

        // Longitude check handles date line crossing
        if Self::crosses_date_line(self.min_longitude, self.max_longitude) {
            // Bounds cross date line: longitude must be >= min OR <= max
            coord.longitude >= self.min_longitude || coord.longitude <= self.max_longitude
        } else {
            // Normal bounds: longitude must be between min and max
            coord.longitude >= self.min_longitude && coord.longitude <= self.max_longitude
        }
    }

    /// Get the center point of these bounds
    ///
    /// # Errors
    ///
    /// Returns `GpsError` if calculated center coordinates are invalid.
    pub fn center(&self) -> Result<GpsCoordinate, GpsError> {
        let lat = (self.min_latitude + self.max_latitude) / 2.0;

        // Handle date line crossing for longitude
        let lon = if Self::crosses_date_line(self.min_longitude, self.max_longitude) {
            // Average across date line
            let adjusted_max = self.max_longitude + 360.0;
            let avg = (self.min_longitude + adjusted_max) / 2.0;
            if avg > 180.0 {
                avg - 360.0
            } else {
                avg
            }
        } else {
            (self.min_longitude + self.max_longitude) / 2.0
        };

        GpsCoordinate::at_sea_level(lat, lon)
    }
}

/// A geographic zone with hierarchy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicZone {
    /// Unique identifier for this zone
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Hierarchy level
    pub level: GeographicLevel,
    /// GPS bounds of this zone
    pub bounds: GeographicBounds,
    /// Parent zone ID (None for global)
    pub parent_id: Option<String>,
    /// Child zone IDs
    pub child_ids: HashSet<String>,
    /// Additional metadata (e.g., population, timezone)
    pub metadata: HashMap<String, String>,
}

impl GeographicZone {
    /// Create a new geographic zone
    pub fn new(
        id: String,
        name: String,
        level: GeographicLevel,
        bounds: GeographicBounds,
    ) -> Self {
        Self {
            id,
            name,
            level,
            bounds,
            parent_id: None,
            child_ids: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Check if a GPS coordinate is within this zone
    pub fn contains(&self, coord: &GpsCoordinate) -> bool {
        self.bounds.contains(coord)
    }

    /// Add a child zone
    pub fn add_child(&mut self, child_id: String) {
        self.child_ids.insert(child_id);
    }

    /// Remove a child zone
    pub fn remove_child(&mut self, child_id: &str) -> bool {
        self.child_ids.remove(child_id)
    }

    /// Set parent zone
    pub fn set_parent(&mut self, parent_id: String) {
        self.parent_id = Some(parent_id);
    }
}

/// Geographic hierarchy manager
#[derive(Debug, Default)]
pub struct GeographicHierarchy {
    /// All zones indexed by ID
    zones: HashMap<String, GeographicZone>,
    /// Index zones by level for efficient queries
    zones_by_level: HashMap<GeographicLevel, HashSet<String>>,
}

impl GeographicHierarchy {
    /// Create a new empty hierarchy
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize with pre-populated data
    pub fn with_defaults() -> Self {
        let mut hierarchy = Self::new();
        hierarchy.populate_defaults();
        hierarchy
    }

    /// Add a zone to the hierarchy
    pub fn add_zone(&mut self, zone: GeographicZone) -> Result<(), String> {
        let zone_id = zone.id.clone();
        let level = zone.level;

        // Validate parent exists if specified
        if let Some(parent_id) = &zone.parent_id {
            if !self.zones.contains_key(parent_id) {
                return Err(format!("Parent zone '{}' not found", parent_id));
            }
        }

        // Add to main index
        self.zones.insert(zone_id.clone(), zone);

        // Add to level index
        self.zones_by_level
            .entry(level)
            .or_insert_with(HashSet::new)
            .insert(zone_id.clone());

        // Update parent's child list
        if let Some(parent_id) = self.zones[&zone_id].parent_id.clone() {
            if let Some(parent) = self.zones.get_mut(&parent_id) {
                parent.add_child(zone_id);
            }
        }

        Ok(())
    }

    /// Remove a zone from the hierarchy
    pub fn remove_zone(&mut self, zone_id: &str) -> Result<GeographicZone, String> {
        let zone = self.zones.remove(zone_id)
            .ok_or_else(|| format!("Zone '{}' not found", zone_id))?;

        // Remove from level index
        if let Some(level_zones) = self.zones_by_level.get_mut(&zone.level) {
            level_zones.remove(zone_id);
        }

        // Remove from parent's child list
        if let Some(parent_id) = &zone.parent_id {
            if let Some(parent) = self.zones.get_mut(parent_id) {
                parent.remove_child(zone_id);
            }
        }

        // Orphan all children (set their parent to None)
        for child_id in &zone.child_ids {
            if let Some(child) = self.zones.get_mut(child_id) {
                child.parent_id = None;
            }
        }

        Ok(zone)
    }

    /// Get a zone by ID
    pub fn get_zone(&self, zone_id: &str) -> Option<&GeographicZone> {
        self.zones.get(zone_id)
    }

    /// Get mutable zone by ID
    pub fn get_zone_mut(&mut self, zone_id: &str) -> Option<&mut GeographicZone> {
        self.zones.get_mut(zone_id)
    }

    /// Find all zones containing a GPS coordinate
    pub fn find_zones_containing(&self, coord: &GpsCoordinate) -> Vec<&GeographicZone> {
        self.zones
            .values()
            .filter(|zone| zone.contains(coord))
            .collect()
    }

    /// Find zones at a specific level containing a coordinate
    pub fn find_zones_at_level(
        &self,
        coord: &GpsCoordinate,
        level: GeographicLevel,
    ) -> Vec<&GeographicZone> {
        if let Some(level_zones) = self.zones_by_level.get(&level) {
            level_zones
                .iter()
                .filter_map(|id| self.zones.get(id))
                .filter(|zone| zone.contains(coord))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all zones at a specific level
    pub fn get_zones_at_level(&self, level: GeographicLevel) -> Vec<&GeographicZone> {
        if let Some(level_zones) = self.zones_by_level.get(&level) {
            level_zones
                .iter()
                .filter_map(|id| self.zones.get(id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get the full hierarchy path for a coordinate (from Global to Local)
    pub fn get_hierarchy_path(&self, coord: &GpsCoordinate) -> Vec<&GeographicZone> {
        let mut path: Vec<&GeographicZone> = Vec::new();
        let mut zones = self.find_zones_containing(coord);

        // Sort by level (Global first, Local last)
        zones.sort_by_key(|z| z.level);

        // Build path ensuring parent-child relationships
        for zone in zones {
            let should_add = if path.is_empty() {
                true
            } else if let Some(last_zone) = path.last() {
                last_zone.child_ids.contains(&zone.id)
            } else {
                false
            };

            if should_add {
                path.push(zone);
            }
        }

        path
    }

    /// Populate with default geographic data
    fn populate_defaults(&mut self) {
        // Add global zone - these bounds are always valid
        let global_bounds = match GeographicBounds::new(-90.0, 90.0, -180.0, 180.0) {
            Ok(bounds) => bounds,
            Err(_) => return, // Should never happen with valid coordinates
        };
        let global = GeographicZone::new(
            "global".to_string(),
            "Global".to_string(),
            GeographicLevel::Global,
            global_bounds,
        );
        if let Err(_) = self.add_zone(global) {
            return; // If global fails, can't continue
        }

        // Add continents
        self.add_continent("north_america", "North America", 15.0, 72.0, -168.0, -52.0);
        self.add_continent("south_america", "South America", -56.0, 13.0, -82.0, -34.0);
        self.add_continent("europe", "Europe", 36.0, 71.0, -10.0, 40.0);
        self.add_continent("africa", "Africa", -35.0, 37.0, -17.0, 51.0);
        self.add_continent("asia", "Asia", 1.0, 77.0, 26.0, 180.0);
        self.add_continent("oceania", "Oceania", -47.0, -10.0, 112.0, 180.0);

        // Add major countries
        self.add_country("usa", "United States", "north_america", 24.5, 49.4, -125.0, -66.9);
        self.add_country("canada", "Canada", "north_america", 41.7, 83.1, -141.0, -52.6);
        self.add_country("mexico", "Mexico", "north_america", 14.5, 32.7, -118.4, -86.7);
        self.add_country("brazil", "Brazil", "south_america", -33.8, 5.3, -73.9, -34.8);
        self.add_country("uk", "United Kingdom", "europe", 49.9, 60.8, -8.6, 1.8);
        self.add_country("germany", "Germany", "europe", 47.3, 55.0, 5.9, 15.0);
        self.add_country("france", "France", "europe", 41.3, 51.1, -5.1, 9.6);
        self.add_country("china", "China", "asia", 18.2, 53.6, 73.6, 134.8);
        self.add_country("japan", "Japan", "asia", 24.0, 46.0, 123.0, 146.0);
        self.add_country("india", "India", "asia", 8.1, 35.5, 68.2, 97.4);
        self.add_country("australia", "Australia", "oceania", -43.6, -10.7, 113.3, 153.6);

        // Add some major cities
        self.add_city("nyc", "New York City", "usa", 40.4774, 40.9176, -74.2591, -73.7002);
        self.add_city("la", "Los Angeles", "usa", 33.7037, 34.3373, -118.6682, -117.9886);
        self.add_city("london", "London", "uk", 51.2868, 51.6919, -0.5103, 0.3340);
        self.add_city("paris", "Paris", "france", 48.8156, 48.9022, 2.2241, 2.4699);
        self.add_city("berlin", "Berlin", "germany", 52.3382, 52.6755, 13.0883, 13.7612);
        self.add_city("tokyo", "Tokyo", "japan", 35.5329, 35.8174, 139.5651, 139.9213);
        self.add_city("sydney", "Sydney", "australia", -34.0183, -33.5781, 150.7108, 151.3430);
    }

    /// Helper to add continent
    fn add_continent(&mut self, id: &str, name: &str, min_lat: f64, max_lat: f64,
                     min_lon: f64, max_lon: f64) {
        let bounds = match GeographicBounds::new(min_lat, max_lat, min_lon, max_lon) {
            Ok(b) => b,
            Err(_) => return, // Skip invalid bounds
        };
        let mut zone = GeographicZone::new(
            id.to_string(),
            name.to_string(),
            GeographicLevel::Continent,
            bounds,
        );
        zone.set_parent("global".to_string());
        let _ = self.add_zone(zone); // Ignore errors in default population
    }

    /// Helper to add country
    fn add_country(&mut self, id: &str, name: &str, continent: &str,
                   min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) {
        let bounds = match GeographicBounds::new(min_lat, max_lat, min_lon, max_lon) {
            Ok(b) => b,
            Err(_) => return, // Skip invalid bounds
        };
        let mut zone = GeographicZone::new(
            id.to_string(),
            name.to_string(),
            GeographicLevel::Country,
            bounds,
        );
        zone.set_parent(continent.to_string());
        let _ = self.add_zone(zone); // Ignore errors in default population
    }

    /// Helper to add city
    fn add_city(&mut self, id: &str, name: &str, country: &str,
                min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) {
        let bounds = match GeographicBounds::new(min_lat, max_lat, min_lon, max_lon) {
            Ok(b) => b,
            Err(_) => return, // Skip invalid bounds
        };
        let mut zone = GeographicZone::new(
            id.to_string(),
            name.to_string(),
            GeographicLevel::City,
            bounds,
        );
        zone.set_parent(country.to_string());
        let _ = self.add_zone(zone); // Ignore errors in default population
    }

    /// Get total number of zones
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    /// Get statistics about the hierarchy
    pub fn get_statistics(&self) -> HashMap<GeographicLevel, usize> {
        let mut stats = HashMap::new();
        for (level, zones) in &self.zones_by_level {
            stats.insert(*level, zones.len());
        }
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geographic_level_hierarchy() {
        assert_eq!(GeographicLevel::Country.parent_level(), Some(GeographicLevel::Continent));
        assert_eq!(GeographicLevel::Country.child_level(), Some(GeographicLevel::Region));
        assert_eq!(GeographicLevel::Global.parent_level(), None);
        assert_eq!(GeographicLevel::Local.child_level(), None);
    }

    #[test]
    fn test_geographic_bounds() {
        let bounds = GeographicBounds::new(40.0, 50.0, -80.0, -70.0).unwrap();

        // Point inside bounds
        let inside = GpsCoordinate::at_sea_level(45.0, -75.0).unwrap();
        assert!(bounds.contains(&inside));

        // Point outside bounds (latitude)
        let outside_lat = GpsCoordinate::at_sea_level(55.0, -75.0).unwrap();
        assert!(!bounds.contains(&outside_lat));

        // Point outside bounds (longitude)
        let outside_lon = GpsCoordinate::at_sea_level(45.0, -60.0).unwrap();
        assert!(!bounds.contains(&outside_lon));
    }

    #[test]
    fn test_date_line_crossing() {
        // Bounds crossing the date line
        let bounds = GeographicBounds::new(-10.0, 10.0, 170.0, -170.0).unwrap();

        // Point on west side of date line
        let west = GpsCoordinate::at_sea_level(0.0, 175.0).unwrap();
        assert!(bounds.contains(&west));

        // Point on east side of date line
        let east = GpsCoordinate::at_sea_level(0.0, -175.0).unwrap();
        assert!(bounds.contains(&east));

        // Point not in bounds
        let outside = GpsCoordinate::at_sea_level(0.0, 0.0).unwrap();
        assert!(!bounds.contains(&outside));
    }

    #[test]
    fn test_bounds_center() {
        let bounds = GeographicBounds::new(40.0, 50.0, -80.0, -70.0).unwrap();
        let center = bounds.center().unwrap();
        assert_eq!(center.latitude, 45.0);
        assert_eq!(center.longitude, -75.0);

        // Date line crossing
        let bounds2 = GeographicBounds::new(-10.0, 10.0, 170.0, -170.0).unwrap();
        let center2 = bounds2.center().unwrap();
        assert_eq!(center2.latitude, 0.0);
        assert_eq!(center2.longitude, 180.0);
    }

    #[test]
    fn test_zone_creation() {
        let bounds = GeographicBounds::new(40.0, 50.0, -80.0, -70.0).unwrap();
        let mut zone = GeographicZone::new(
            "test_zone".to_string(),
            "Test Zone".to_string(),
            GeographicLevel::Region,
            bounds,
        );

        assert_eq!(zone.id, "test_zone");
        assert_eq!(zone.name, "Test Zone");
        assert_eq!(zone.level, GeographicLevel::Region);
        assert!(zone.parent_id.is_none());
        assert!(zone.child_ids.is_empty());

        // Test parent/child relationships
        zone.set_parent("parent".to_string());
        assert_eq!(zone.parent_id, Some("parent".to_string()));

        zone.add_child("child1".to_string());
        zone.add_child("child2".to_string());
        assert_eq!(zone.child_ids.len(), 2);
        assert!(zone.child_ids.contains("child1"));

        assert!(zone.remove_child("child1"));
        assert_eq!(zone.child_ids.len(), 1);
    }

    #[test]
    fn test_hierarchy_management() {
        let mut hierarchy = GeographicHierarchy::new();

        // Add global zone
        let global = GeographicZone::new(
            "global".to_string(),
            "Global".to_string(),
            GeographicLevel::Global,
            GeographicBounds::new(-90.0, 90.0, -180.0, 180.0).unwrap(),
        );
        hierarchy.add_zone(global).unwrap();

        // Add continent with parent
        let mut continent = GeographicZone::new(
            "continent1".to_string(),
            "Continent 1".to_string(),
            GeographicLevel::Continent,
            GeographicBounds::new(0.0, 50.0, -100.0, -50.0).unwrap(),
        );
        continent.set_parent("global".to_string());
        hierarchy.add_zone(continent).unwrap();

        // Verify parent-child relationship
        let global_zone = hierarchy.get_zone("global").unwrap();
        assert!(global_zone.child_ids.contains("continent1"));

        let continent_zone = hierarchy.get_zone("continent1").unwrap();
        assert_eq!(continent_zone.parent_id, Some("global".to_string()));

        // Test zone removal
        let removed = hierarchy.remove_zone("continent1").unwrap();
        assert_eq!(removed.id, "continent1");
        assert!(hierarchy.get_zone("continent1").is_none());
    }

    #[test]
    fn test_find_zones_containing() {
        let hierarchy = GeographicHierarchy::with_defaults();

        // New York coordinates
        let nyc = GpsCoordinate::at_sea_level(40.7128, -74.0060).unwrap();
        let zones = hierarchy.find_zones_containing(&nyc);

        // Should be in global, north america, usa, and nyc
        assert!(zones.len() >= 4);

        let zone_names: Vec<&str> = zones.iter().map(|z| z.id.as_str()).collect();
        assert!(zone_names.contains(&"global"));
        assert!(zone_names.contains(&"north_america"));
        assert!(zone_names.contains(&"usa"));
        assert!(zone_names.contains(&"nyc"));
    }

    #[test]
    fn test_find_zones_at_level() {
        let hierarchy = GeographicHierarchy::with_defaults();

        // Tokyo coordinates
        let tokyo = GpsCoordinate::at_sea_level(35.6762, 139.6503).unwrap();

        // Find country containing Tokyo
        let countries = hierarchy.find_zones_at_level(&tokyo, GeographicLevel::Country);
        assert_eq!(countries.len(), 1);
        assert_eq!(countries[0].id, "japan");

        // Find city
        let cities = hierarchy.find_zones_at_level(&tokyo, GeographicLevel::City);
        assert_eq!(cities.len(), 1);
        assert_eq!(cities[0].id, "tokyo");
    }

    #[test]
    fn test_hierarchy_path() {
        let hierarchy = GeographicHierarchy::with_defaults();

        // London coordinates
        let london = GpsCoordinate::at_sea_level(51.5074, -0.1278).unwrap();
        let path = hierarchy.get_hierarchy_path(&london);

        assert!(path.len() >= 4); // global, europe, uk, london
        assert_eq!(path[0].level, GeographicLevel::Global);
        assert_eq!(path[1].level, GeographicLevel::Continent);
        assert_eq!(path[2].level, GeographicLevel::Country);
        assert_eq!(path[3].level, GeographicLevel::City);
    }

    #[test]
    fn test_default_hierarchy_statistics() {
        let hierarchy = GeographicHierarchy::with_defaults();
        let stats = hierarchy.get_statistics();

        assert_eq!(stats[&GeographicLevel::Global], 1);
        assert!(stats[&GeographicLevel::Continent] >= 6);
        assert!(stats[&GeographicLevel::Country] >= 11);
        assert!(stats[&GeographicLevel::City] >= 7);

        // Total zones
        assert!(hierarchy.zone_count() >= 25);
    }
}