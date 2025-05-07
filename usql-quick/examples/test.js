import { Pool } from "usql";

const pool = await Pool.open({
  type: "sqlite",
});

const conn = await pool.get();

await conn.transaction(async (conn) => {
  await conn.exec("CREATE TABLE users (name TEST)");
  await conn.exec("INSERT INTO users VALUES (?)", ["Rasmus"]);
});

const rows = await conn.query("SELECT * FROM users");

for (const row of rows) {
  console.log(JSON.stringify(row));
}
