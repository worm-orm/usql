use alloc::fmt;
use bytes::Bytes;
use geozero::{CoordDimensions, ToGeo, ToWkb, ToWkt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EPSG(i32);

impl EPSG {
    pub const WEB_MERCATOR: EPSG = EPSG(3857);
    pub const WGS_84: EPSG = EPSG(4326);

    pub fn new(n: i32) -> EPSG {
        EPSG(n)
    }
}

pub struct Geometry(Bytes);

impl Geometry {
    pub fn new(
        srid: EPSG,
        geometry: impl Into<geo_types::Geometry>,
    ) -> Result<Geometry, geozero::error::GeozeroError> {
        let geo = geometry.into();
        let ret = geo.to_ewkb(CoordDimensions::xy(), Some(srid.0))?;
        Ok(Self(ret.into()))
    }

    pub fn geom(&self) -> Result<geo_types::Geometry, geozero::error::GeozeroError> {
        geozero::wkb::Ewkb(&self.0).to_geo()
    }
}

impl fmt::Display for Geometry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let out = geozero::wkb::Ewkb(&self.0)
            .to_ewkt(None)
            .map_err(|_| fmt::Error)?;
        write!(f, "{}", out)
    }
}
