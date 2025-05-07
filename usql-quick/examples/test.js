import { Pool } from "usql";

const pool = await Pool.open({
  type: "sqlite",
});

const conn = await pool.get();

await conn.query("SELECT * FROM user");
