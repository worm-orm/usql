use geozero::{CoordDimensions, ToJson, ToWkb, ToWkt};

fn main() {
    let w = "SRID=4326;POINT(-44.3 60.1)";

    let out = geozero::wkt::Wkt("POINT(-44.3 60.1)");
    let bs = out.to_ewkb(CoordDimensions::xy(), Some(4326)).unwrap();

    let out = geozero::wkb::Ewkb(bs).to_json(); //.to_ewkt(None);

    println!("{:?}", out);
}
