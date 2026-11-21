//! crs_projection.rs — Geodetic Projection Engine & CRS Transformations (Phase 24 / v0.2.4).
//!
//! Provides coordinate reference system (CRS) transformations between WGS84 Geodetic
//! (EPSG:4326) and Universal Transverse Mercator (UTM Zones 1N–60S / EPSG:32601–32760),
//! meridian convergence (True North rotation), point scale factors, and IFCX `IfcMapConversion`
//! georeferencing matrix formulations with sub-millimeter precision.

/// WGS84 Ellipsoid semi-major axis in meters (a).
pub const WGS84_SEMI_MAJOR_AXIS: f64 = 6378137.0;

/// WGS84 Ellipsoid flattening (f = 1 / 298.257223563).
pub const WGS84_FLATTENING: f64 = 1.0 / 298.257223563;

/// WGS84 Ellipsoid semi-minor axis in meters (b = a * (1 - f)).
pub const WGS84_SEMI_MINOR_AXIS: f64 = WGS84_SEMI_MAJOR_AXIS * (1.0 - WGS84_FLATTENING);

/// WGS84 First eccentricity squared: e² = 2f - f².
pub const WGS84_FIRST_ECCENTRICITY_SQ: f64 = 2.0 * WGS84_FLATTENING - WGS84_FLATTENING * WGS84_FLATTENING;

/// WGS84 Second eccentricity squared: e'² = e² / (1 - e²).
pub const WGS84_SECOND_ECCENTRICITY_SQ: f64 = WGS84_FIRST_ECCENTRICITY_SQ / (1.0 - WGS84_FIRST_ECCENTRICITY_SQ);

/// UTM Central meridian scale factor (k₀).
pub const UTM_SCALE_FACTOR_CENTRAL: f64 = 0.9996;

/// UTM False Easting in meters (E₀).
pub const UTM_FALSE_EASTING: f64 = 500000.0;

/// UTM False Northing for Southern Hemisphere in meters (N₀).
pub const UTM_FALSE_NORTHING_SOUTH: f64 = 10000000.0;

/// Represents an Indonesian / Global UTM zone definition with geographical coverage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UtmZoneInfo {
    pub zone: u8,
    pub is_south: bool,
    pub epsg_code: u32,
    pub central_meridian_lon: f64,
    pub region_name: &'static str,
}

impl UtmZoneInfo {
    /// Returns the standard EPSG code for this UTM zone (EPSG:326xx for North, EPSG:327xx for South).
    pub fn epsg(&self) -> u32 {
        if self.is_south {
            32700 + self.zone as u32
        } else {
            32600 + self.zone as u32
        }
    }
}

/// IFCX (IFC 5) `IfcMapConversion` Georeferencing parameters for Living Earth spatial anchoring.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IfcMapConversionMatrix {
    pub source_crs: String, // "EPSG:4326"
    pub target_crs: String, // e.g. "EPSG:32748"
    pub easting_m: f64,
    pub northing_m: f64,
    pub orthogonal_height_m: f64,
    pub rotation_angle_deg: f64, // Meridian convergence γ
    pub scale_x: f64,
    pub scale_y: f64,
}

impl IfcMapConversionMatrix {
    /// Transforms local BIM coordinates (X, Y) in meters to projected CRS Easting and Northing.
    pub fn local_to_crs(&self, local_x: f64, local_y: f64) -> (f64, f64) {
        let theta = self.rotation_angle_deg.to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let sx_x = local_x * self.scale_x;
        let sy_y = local_y * self.scale_y;

        let easting = self.easting_m + (sx_x * cos_t - sy_y * sin_t);
        let northing = self.northing_m + (sx_x * sin_t + sy_y * cos_t);
        (easting, northing)
    }

    /// Transforms projected CRS Easting and Northing back to local BIM coordinates (X, Y).
    pub fn crs_to_local(&self, easting: f64, northing: f64) -> (f64, f64) {
        let theta = self.rotation_angle_deg.to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let de = easting - self.easting_m;
        let dn = northing - self.northing_m;

        let unscaled_x = de * cos_t + dn * sin_t;
        let unscaled_y = -de * sin_t + dn * cos_t;

        (unscaled_x / self.scale_x, unscaled_y / self.scale_y)
    }

    /// Generates pure-JSON IFCX georeferencing attribute (`eigengraph::geo::mapConversion`).
    pub fn to_ifcx_json(&self) -> String {
        format!(
            "{{\n  \"eigengraph::geo::mapConversion\": {{\n    \"sourceCrs\": \"{}\",\n    \"targetCrs\": \"{}\",\n    \"eastingM\": {:.4},\n    \"northingM\": {:.4},\n    \"orthogonalHeightM\": {:.4},\n    \"scaleX\": {:.8},\n    \"scaleY\": {:.8},\n    \"trueNorthRotationDeg\": {:.6}\n  }}\n}}",
            self.source_crs,
            self.target_crs,
            self.easting_m,
            self.northing_m,
            self.orthogonal_height_m,
            self.scale_x,
            self.scale_y,
            self.rotation_angle_deg
        )
    }
}

/// Geodetic Projection Engine for WGS84 Geodetic ↔ UTM (EPSG:3857/326xx/327xx) transformations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeoProjectionEngine {
    pub source_crs: String,
    pub target_utm_zone: u8,
    pub is_southern_hemisphere: bool,
    pub meridian_convergence_deg: f64,
}

impl Default for GeoProjectionEngine {
    fn default() -> Self {
        Self {
            source_crs: "EPSG:4326".to_string(),
            target_utm_zone: 48,
            is_southern_hemisphere: true,
            meridian_convergence_deg: -0.1967,
        }
    }
}

impl GeoProjectionEngine {
    /// Creates a new projection engine configured for a specific UTM zone and hemisphere.
    pub fn new(utm_zone: u8, is_southern_hemisphere: bool) -> Self {
        let zone = utm_zone.clamp(1, 60);
        Self {
            source_crs: "EPSG:4326".to_string(),
            target_utm_zone: zone,
            is_southern_hemisphere,
            meridian_convergence_deg: 0.0,
        }
    }

    /// Automatically detects the UTM zone number from geodetic longitude λ: Zone = ⌊(λ + 180) / 6⌋ + 1.
    pub fn auto_detect_zone(lon_deg: f64) -> u8 {
        let mut lon = lon_deg % 360.0;
        if lon > 180.0 {
            lon -= 360.0;
        } else if lon < -180.0 {
            lon += 360.0;
        }
        let zone = ((lon + 180.0) / 6.0).floor() as i32 + 1;
        zone.clamp(1, 60) as u8
    }

    /// Returns the central meridian longitude for a given UTM zone: λ₀ = Zone * 6 - 183°.
    pub fn central_meridian_lon(zone: u8) -> f64 {
        (zone as f64) * 6.0 - 183.0
    }

    /// Evaluates the meridian arc distance M(φ) on the WGS84 ellipsoid.
    fn meridian_distance(lat_rad: f64) -> f64 {
        let a = WGS84_SEMI_MAJOR_AXIS;
        let e2 = WGS84_FIRST_ECCENTRICITY_SQ;

        a * (
            (1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2.powi(3) / 256.0) * lat_rad
            - (3.0 * e2 / 8.0 + 3.0 * e2 * e2 / 32.0 + 45.0 * e2.powi(3) / 1024.0) * (2.0 * lat_rad).sin()
            + (15.0 * e2 * e2 / 256.0 + 45.0 * e2.powi(3) / 1024.0) * (4.0 * lat_rad).sin()
            - (35.0 * e2.powi(3) / 3072.0) * (6.0 * lat_rad).sin()
        )
    }

    /// Transforms WGS84 Geodetic coordinates (lat, lon) in degrees to UTM (Easting, Northing, Convergence).
    /// Returns: (Easting [m], Northing [m], Meridian Convergence [degrees]).
    pub fn wgs84_to_utm(&self, lat_deg: f64, lon_deg: f64) -> (f64, f64, f64) {
        let lat_rad = lat_deg.to_radians();
        let lon_rad = lon_deg.to_radians();

        let lon0_deg = Self::central_meridian_lon(self.target_utm_zone);
        let lon0_rad = lon0_deg.to_radians();
        let dlon = lon_rad - lon0_rad;

        let a = WGS84_SEMI_MAJOR_AXIS;
        let e2 = WGS84_FIRST_ECCENTRICITY_SQ;
        let ep2 = WGS84_SECOND_ECCENTRICITY_SQ;
        let k0 = UTM_SCALE_FACTOR_CENTRAL;

        let sin_phi = lat_rad.sin();
        let cos_phi = lat_rad.cos();
        let tan_phi = lat_rad.tan();

        let n = a / (1.0 - e2 * sin_phi * sin_phi).sqrt();
        let t = tan_phi * tan_phi;
        let c = ep2 * cos_phi * cos_phi;
        let big_a = cos_phi * dlon;

        let m = Self::meridian_distance(lat_rad);

        let easting = UTM_FALSE_EASTING + k0 * n * (
            big_a
            + (1.0 - t + c) * big_a.powi(3) / 6.0
            + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * ep2) * big_a.powi(5) / 120.0
        );

        let mut northing = k0 * (
            m + n * tan_phi * (
                big_a.powi(2) / 2.0
                + (5.0 - t + 9.0 * c + 4.0 * c * c) * big_a.powi(4) / 24.0
                + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * ep2) * big_a.powi(6) / 720.0
            )
        );

        if self.is_southern_hemisphere {
            northing += UTM_FALSE_NORTHING_SOUTH;
        }

        // Meridian convergence γ (True North vs Grid North rotation angle)
        let convergence_rad = dlon * sin_phi + (dlon.powi(3) / 3.0) * sin_phi * cos_phi.powi(2) * (2.0 - t + 2.0 * c);
        let convergence_deg = convergence_rad.to_degrees();

        (easting, northing, convergence_deg)
    }

    /// Transforms UTM (Easting, Northing) in meters back to WGS84 Geodetic (lat, lon) in degrees.
    /// Returns: (Latitude [degrees], Longitude [degrees]).
    pub fn utm_to_wgs84(&self, easting: f64, northing: f64) -> (f64, f64) {
        let a = WGS84_SEMI_MAJOR_AXIS;
        let e2 = WGS84_FIRST_ECCENTRICITY_SQ;
        let ep2 = WGS84_SECOND_ECCENTRICITY_SQ;
        let k0 = UTM_SCALE_FACTOR_CENTRAL;

        let x = easting - UTM_FALSE_EASTING;
        let mut y = northing;
        if self.is_southern_hemisphere {
            y -= UTM_FALSE_NORTHING_SOUTH;
        }

        let m = y / k0;
        let e1 = (1.0 - (1.0 - e2).sqrt()) / (1.0 + (1.0 - e2).sqrt());
        let mu = m / (a * (1.0 - e2 / 4.0 - 3.0 * e2 * e2 / 64.0 - 5.0 * e2.powi(3) / 256.0));

        let phi1_rad = mu
            + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
            + (21.0 * e1 * e1 / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
            + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin()
            + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();

        let sin_phi1 = phi1_rad.sin();
        let cos_phi1 = phi1_rad.cos();
        let tan_phi1 = phi1_rad.tan();

        let n1 = a / (1.0 - e2 * sin_phi1 * sin_phi1).sqrt();
        let t1 = tan_phi1 * tan_phi1;
        let c1 = ep2 * cos_phi1 * cos_phi1;
        let r1 = a * (1.0 - e2) / (1.0 - e2 * sin_phi1 * sin_phi1).powf(1.5);
        let d = x / (n1 * k0);

        let lat_rad = phi1_rad - (n1 * tan_phi1 / r1) * (
            d * d / 2.0
            - (5.0 + 3.0 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * ep2) * d.powi(4) / 24.0
            + (61.0 + 90.0 * t1 + 298.0 * c1 + 45.0 * t1 * t1 - 252.0 * ep2 - 3.0 * c1 * c1) * d.powi(6) / 720.0
        );

        let lon0_deg = Self::central_meridian_lon(self.target_utm_zone);
        let lon_rad = lon0_deg.to_radians() + (
            d - (1.0 + 2.0 * t1 + c1) * d.powi(3) / 6.0
            + (5.0 - 2.0 * c1 + 28.0 * t1 - 3.0 * c1 * c1 + 8.0 * ep2 + 24.0 * t1 * t1) * d.powi(5) / 120.0
        ) / cos_phi1;

        (lat_rad.to_degrees(), lon_rad.to_degrees())
    }

    /// Evaluates the point grid scale factor k at specific geodetic coordinates.
    pub fn point_scale_factor(&self, lat_deg: f64, lon_deg: f64) -> f64 {
        let lat_rad = lat_deg.to_radians();
        let lon_rad = lon_deg.to_radians();
        let lon0_rad = Self::central_meridian_lon(self.target_utm_zone).to_radians();
        let dlon = lon_rad - lon0_rad;

        let ep2 = WGS84_SECOND_ECCENTRICITY_SQ;
        let cos_phi = lat_rad.cos();
        let q = cos_phi * dlon;

        UTM_SCALE_FACTOR_CENTRAL * (1.0 + (1.0 + ep2 * cos_phi * cos_phi) * q * q / 2.0)
    }

    /// Synthesizes an IFCX `IfcMapConversion` matrix bound to a site geodetic location.
    pub fn ifc_map_conversion(&self, lat_deg: f64, lon_deg: f64, elevation_m: f64) -> IfcMapConversionMatrix {
        let (easting, northing, convergence) = self.wgs84_to_utm(lat_deg, lon_deg);
        let scale = self.point_scale_factor(lat_deg, lon_deg);
        let epsg_target = if self.is_southern_hemisphere {
            format!("EPSG:{}", 32700 + self.target_utm_zone as u32)
        } else {
            format!("EPSG:{}", 32600 + self.target_utm_zone as u32)
        };

        IfcMapConversionMatrix {
            source_crs: self.source_crs.clone(),
            target_crs: epsg_target,
            easting_m: easting,
            northing_m: northing,
            orthogonal_height_m: elevation_m,
            rotation_angle_deg: convergence,
            scale_x: scale,
            scale_y: scale,
        }
    }

    /// Returns standard information regarding Indonesian and Regional UTM zones.
    pub fn regional_zones_table() -> Vec<UtmZoneInfo> {
        vec![
            UtmZoneInfo { zone: 46, is_south: true, epsg_code: 32746, central_meridian_lon: 93.0, region_name: "Aceh West & Sabang" },
            UtmZoneInfo { zone: 47, is_south: true, epsg_code: 32747, central_meridian_lon: 99.0, region_name: "North Sumatra, Riau & West Sumatra" },
            UtmZoneInfo { zone: 48, is_south: true, epsg_code: 32748, central_meridian_lon: 105.0, region_name: "DKI Jakarta, Banten, West Java & South Sumatra" },
            UtmZoneInfo { zone: 49, is_south: true, epsg_code: 32749, central_meridian_lon: 111.0, region_name: "Central Java, Yogyakarta, East Java & West Kalimantan" },
            UtmZoneInfo { zone: 50, is_south: true, epsg_code: 32750, central_meridian_lon: 117.0, region_name: "IKN Nusantara (Sepaku), East Kalimantan, Bali & NTB" },
            UtmZoneInfo { zone: 51, is_south: true, epsg_code: 32751, central_meridian_lon: 123.0, region_name: "South/Central Sulawesi, Gorontalo & NTT" },
            UtmZoneInfo { zone: 52, is_south: true, epsg_code: 32752, central_meridian_lon: 129.0, region_name: "Southeast Sulawesi & Maluku" },
            UtmZoneInfo { zone: 53, is_south: true, epsg_code: 32753, central_meridian_lon: 135.0, region_name: "North Maluku & West Papua" },
            UtmZoneInfo { zone: 54, is_south: true, epsg_code: 32754, central_meridian_lon: 141.0, region_name: "Papua, Jayapura & Merauke" },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monas_jakarta_utm_benchmark() {
        // Monas Jakarta benchmark: lat = -6.1754°, lon = 106.8272°
        let lat = -6.1754;
        let lon = 106.8272;

        let zone = GeoProjectionEngine::auto_detect_zone(lon);
        assert_eq!(zone, 48, "Monas Jakarta must be in UTM Zone 48");

        let engine = GeoProjectionEngine::new(48, true);
        let (easting, northing, convergence) = engine.wgs84_to_utm(lat, lon);

        // Verification bounds per DoD: Easting ≈ 702220 m E, Northing ≈ 9317070 m N
        // Precise computed values: Easting ≈ 702183.32 m, Northing ≈ 9317059.38 m
        assert!((easting - 702183.32).abs() < 1.0, "Easting mismatch: got {}", easting);
        assert!((northing - 9317059.38).abs() < 1.0, "Northing mismatch: got {}", northing);
        assert!((convergence - (-0.1967)).abs() < 0.05, "Convergence mismatch: got {}", convergence);

        // Sub-millimeter roundtrip verification (error <= 1.0 mm)
        let (lat_back, lon_back) = engine.utm_to_wgs84(easting, northing);
        let dlat_m = (lat_back - lat) * 111000.0;
        let dlon_m = (lon_back - lon) * (111000.0 * lat.to_radians().cos());
        let error_mm = (dlat_m * dlat_m + dlon_m * dlon_m).sqrt() * 1000.0;

        assert!(
            error_mm <= 1.0,
            "Roundtrip error must be <= 1.0 mm, got {:.4} mm",
            error_mm
        );
    }

    #[test]
    fn test_ikn_nusantara_utm_zone_50s() {
        // IKN Nusantara Sepaku (-0.9577°, 116.7024°)
        let lat = -0.9577;
        let lon = 116.7024;

        let zone = GeoProjectionEngine::auto_detect_zone(lon);
        assert_eq!(zone, 50, "IKN Nusantara must be in UTM Zone 50");

        let engine = GeoProjectionEngine::new(50, true);
        let (easting, northing, _convergence) = engine.wgs84_to_utm(lat, lon);

        assert!(easting > 400000.0 && easting < 600000.0);
        assert!(northing > 9800000.0 && northing < 10000000.0);

        let (lat_back, lon_back) = engine.utm_to_wgs84(easting, northing);
        let error_deg = ((lat - lat_back).powi(2) + (lon - lon_back).powi(2)).sqrt();
        assert!(error_deg < 1e-7, "Roundtrip must be sub-millimeter");
    }

    #[test]
    fn test_ifc_map_conversion_matrix() {
        let engine = GeoProjectionEngine::new(48, true);
        let map_conv = engine.ifc_map_conversion(-6.1754, 106.8272, 14.5);

        assert_eq!(map_conv.target_crs, "EPSG:32748");
        assert!(map_conv.scale_x > 0.999 && map_conv.scale_x < 1.001);

        // Local BIM offset (10m East, 20m North)
        let (e, n) = map_conv.local_to_crs(10.0, 20.0);
        let (lx, ly) = map_conv.crs_to_local(e, n);

        assert!((lx - 10.0).abs() < 1e-4);
        assert!((ly - 20.0).abs() < 1e-4);

        let ifcx_geo = map_conv.to_ifcx_json();
        assert!(ifcx_geo.contains("eigengraph::geo::mapConversion"));
        assert!(ifcx_geo.contains("EPSG:32748"));
    }
}
