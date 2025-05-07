use rquickjs::Class;
use rquickjs_modules::module_info;

use crate::{JsConn, JsPool, JsStatement};

pub struct Module;

impl rquickjs::module::ModuleDef for Module {
    fn declare<'js>(decl: &rquickjs::module::Declarations<'js>) -> rquickjs::Result<()> {
        decl.declare("Statement")?;
        decl.declare("Conn")?;
        decl.declare("Pool")?;

        Ok(())
    }

    fn evaluate<'js>(
        ctx: &rquickjs::Ctx<'js>,
        exports: &rquickjs::module::Exports<'js>,
    ) -> rquickjs::Result<()> {
        exports.export("Statement", Class::<JsStatement>::create_constructor(ctx)?)?;
        exports.export("Conn", Class::<JsConn>::create_constructor(ctx)?)?;
        exports.export("Pool", Class::<JsPool>::create_constructor(ctx)?)?;
        Ok(())
    }
}

module_info!("usql" => Module);
