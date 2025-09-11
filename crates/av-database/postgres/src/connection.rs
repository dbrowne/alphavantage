use diesel::prelude::*;
use diesel::pg::PgConnection;

/// Establish a database connection
pub fn establish_connection(database_url: &str) -> Result<PgConnection, diesel::ConnectionError> {
    PgConnection::establish(database_url)
}