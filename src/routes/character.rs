use crate::{
  error::*,
  database::DbConn,
  redis::Redis,
};

use chrono::{TimeZone, Utc};

use lodestone_parser::models::character::Character;

use redis::Commands;

use rocket_contrib::Json;

#[get("/character/<id>")]
crate fn get(id: u64, conn: DbConn, redis: Redis) -> Result<Json<RouteResult<Character>>> {
  use diesel::prelude::*;
  use crate::database::{
    models::{U64, characters::DatabaseCharacter},
    schema::characters,
  };
  let db_char: Option<DatabaseCharacter> = characters::table
    .find(U64(id))
    .get_result(&*conn)
    .optional()?;
  if let Some(dbc) = db_char {
    let c: Character = serde_json::from_value(dbc.data)?;
    let new_frecency = crate::frecency::frecency(Some(dbc.frecency));
    diesel::update(characters::table)
      .set(characters::frecency.eq(new_frecency))
      .filter(characters::id.eq(dbc.id))
      .execute(&*conn)?;
    return Ok(Json(RouteResult::Success {
      result: c,
      last_update: Utc.from_utc_datetime(&dbc.last_update),
    }));
  }
  if let Ok(Some((rr, _))) = crate::find_redis(&redis, &format!("character_{}", id)) {
    return Ok(Json(rr));
  }
  if let Some(idx) = redis.hget("character_queue_hash", id)? {
    return Ok(Json(RouteResult::Adding { queue_position: idx }));
  }
  let pos: u64 = redis.rpush("character_queue", id)?;
  redis.hset("character_queue_hash", id, pos)?;
  Ok(Json(RouteResult::Adding { queue_position: pos }))
}
