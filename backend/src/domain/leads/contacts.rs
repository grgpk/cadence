//! The people behind a host's leads and bookings: one row per person and
//! host, found again by email whatever its case.

use sqlx::PgConnection;
use uuid::Uuid;

use super::models::LeadUpdate;

/// The contact a lead's form fills in, after `update`.
///
/// A lead without one gets a new contact. An email another contact of the
/// host already has moves the lead to that contact, which takes what this
/// form typed.
pub async fn save_for_lead(
    connection: &mut PgConnection,
    host_unid: Uuid,
    current: Option<Uuid>,
    update: &LeadUpdate,
) -> Result<Uuid, sqlx::Error> {
    let known = match update.email.as_deref() {
        Some(email) => find_by_email(&mut *connection, host_unid, email).await?,
        None => None,
    };
    let Some(target) = known.or(current) else {
        return insert(connection, host_unid, update).await;
    };
    // A field the form left empty takes the form's earlier contact's value,
    // when the email moved it here, before this contact's own.
    sqlx::query_scalar!(
        "UPDATE contacts c SET
             email = COALESCE($2, c.email),
             first_name = COALESCE($3, (SELECT o.first_name FROM contacts o WHERE o.unid = $7 AND o.unid <> $1), c.first_name),
             last_name = COALESCE($4, (SELECT o.last_name FROM contacts o WHERE o.unid = $7 AND o.unid <> $1), c.last_name),
             phone = COALESCE($5, (SELECT o.phone FROM contacts o WHERE o.unid = $7 AND o.unid <> $1), c.phone),
             country_code = COALESCE($6, (SELECT o.country_code FROM contacts o WHERE o.unid = $7 AND o.unid <> $1), c.country_code),
             updated_at = now()
         WHERE c.unid = $1
         RETURNING 1",
        target,
        update.email.as_deref(),
        update.first_name.as_deref(),
        update.last_name.as_deref(),
        update.phone.as_deref(),
        update.country_code.as_deref(),
        current,
    )
    .fetch_one(&mut *connection)
    .await?;
    Ok(target)
}

/// The contact who books with `email`, made on first booking.
pub async fn for_booking(
    connection: &mut PgConnection,
    host_unid: Uuid,
    name: &str,
    email: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO contacts (host_unid, email, first_name) VALUES ($1, $2, $3)
         ON CONFLICT (host_unid, lower(email)) WHERE email IS NOT NULL
         DO UPDATE SET updated_at = now()
         RETURNING unid",
        host_unid,
        email,
        name,
    )
    .fetch_one(connection)
    .await
}

/// Deletes a contact no lead or booking points to any more.
pub async fn delete_if_unused(
    connection: &mut PgConnection,
    unid: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar!(
        "DELETE FROM contacts c WHERE c.unid = $1
           AND NOT EXISTS (SELECT 1 FROM leads l WHERE l.contact_unid = c.unid)
           AND NOT EXISTS (SELECT 1 FROM bookings b WHERE b.contact_unid = c.unid)
         RETURNING 1",
        unid,
    )
    .fetch_optional(connection)
    .await
    .map(|_| ())
}

async fn find_by_email(
    connection: &mut PgConnection,
    host_unid: Uuid,
    email: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT unid FROM contacts WHERE host_unid = $1 AND lower(email) = lower($2)",
        host_unid,
        email,
    )
    .fetch_optional(connection)
    .await
}

async fn insert(
    connection: &mut PgConnection,
    host_unid: Uuid,
    update: &LeadUpdate,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        "INSERT INTO contacts (host_unid, email, first_name, last_name, phone, country_code)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING unid",
        host_unid,
        update.email.as_deref(),
        update.first_name.as_deref(),
        update.last_name.as_deref(),
        update.phone.as_deref(),
        update.country_code.as_deref(),
    )
    .fetch_one(connection)
    .await
}
