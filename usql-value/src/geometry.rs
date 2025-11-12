use alloc::{fmt, vec::Vec};
use bytes::Bytes;
use chrono::naive::serde::ts_nanoseconds_option::serialize;
use geo_types::Geometry;
use geozero::{CoordDimensions, GeozeroGeometry, ToGeo, ToWkb, ToWkt};

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Geom(Bytes);

impl Geom {
    pub fn new(
        srid: EPSG,
        geometry: impl Into<geo_types::Geometry>,
    ) -> Result<Geom, geozero::error::GeozeroError> {
        let geo = geometry.into();
        let ret = geo.to_ewkb(CoordDimensions::xy(), Some(srid.0))?;
        Ok(Self(ret.into()))
    }

    pub fn from_bytes(bytes: Vec<u8>) {
        geozero::wkb::Decode(bytes);
    }

    pub fn epsg(&self) -> Option<EPSG> {
        self.ewkb().srid().map(EPSG)
    }

    pub fn geometry(&self) -> Result<geo_types::Geometry, geozero::error::GeozeroError> {
        geozero::wkb::Ewkb(&self.0).to_geo()
    }
}

impl Geom {
    pub(crate) fn ewkb(&self) -> geozero::wkb::Ewkb<&'_ Bytes> {
        geozero::wkb::Ewkb(&self.0)
    }
}

impl fmt::Display for Geom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let out = geozero::wkb::Ewkb(&self.0)
            .to_ewkt(None)
            .map_err(|_| fmt::Error)?;
        write!(f, "{}", out)
    }
}

impl serde::Serialize for Geom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let geo = self
            .geometry()
            .map_err(|err| <S::Error as serde::ser::Error>::custom(err))?;
        <Geometry as serde::Serialize>::serialize(&geo, serializer)
    }
}
