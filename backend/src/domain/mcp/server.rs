use axum::http::request::Parts;
use chrono::{DateTime, NaiveDate, Utc};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Extension, wrapper::Parameters},
    model::{
        CallToolResult, ContentBlock, Implementation, ListToolsResult, PaginatedRequestParams,
        ServerCapabilities, ServerConfig, Tool,
    },
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    domain::{
        auth::models::UserRecord,
        availability::{
            models::{AvailabilityRuleInput, HoursRange},
            service as availability,
        },
        bookings::{
            models::{
                Booking, BookingStatus, CreateBookingRequest, NoShowRequest, RescheduleRequest,
            },
            service as bookings,
        },
        emails::service::app_url,
        event_types::{
            models::{EventTypeInput, Question},
            service as event_types,
        },
        host_settings::{
            models::{BookingPage, ConferencingApp, ConferencingUpdate},
            service as host_settings,
        },
        leads::service as leads,
        mcp::models::AccessTokenScope,
    },
    error::{AppError, AppResult},
};

/// The host an access token belongs to and what the token may do, put on
/// the request by `handlers::require_token`.
#[derive(Clone)]
pub struct McpHost {
    pub user: UserRecord,
    pub scope: AccessTokenScope,
}

const INSTRUCTIONS: &str = "Cadence is a booking app. These tools act for one host: their calls \
(bookings), the kinds of call they offer (event types), their weekly hours and date \
overrides, and their booking page. Instants are ISO-8601 in UTC; weekly hours and date \
overrides are in the host's own timezone, which get_profile returns. Cancelling, moving or \
booking a call emails the invitee, so confirm those with the user first.";

/// How many rows a list returns when the agent does not say.
const LIST_DEFAULT: usize = 50;
const LIST_MAX: usize = 200;
/// How a wall-clock time in the host's timezone reads.
const LOCAL_FORMAT: &str = "%Y-%m-%dT%H:%M";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBookings {
    /// Only calls with this status.
    pub status: Option<BookingStatus>,
    /// Only calls starting at or after this instant.
    pub from: Option<DateTime<Utc>>,
    /// Only calls starting before this instant.
    pub to: Option<DateTime<Utc>>,
    /// At most this many calls, latest first. 50 when left out, at most 200.
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BookingRef {
    pub booking_unid: Uuid,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NoShow {
    pub booking_unid: Uuid,
    /// `true` marks that the invitee did not come; `false` takes it back.
    pub no_show: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Reschedule {
    pub booking_unid: Uuid,
    /// The new start: one of the times `find_open_slots` returns.
    pub slot_start: DateTime<Utc>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindSlots {
    pub event_type_unid: Uuid,
    /// Only times at or after this instant.
    pub from: Option<DateTime<Utc>>,
    /// Only times before this instant.
    pub to: Option<DateTime<Utc>>,
    /// At most this many times, earliest first. 50 when left out, at most 200.
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateBooking {
    pub event_type_unid: Uuid,
    /// One of the times `find_open_slots` returns.
    pub slot_start: DateTime<Utc>,
    pub invitee_name: String,
    pub invitee_email: String,
    /// The invitee's IANA timezone, for their emails. The host's when left out.
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EventTypeRef {
    pub event_type_unid: Uuid,
}

/// The fields of a kind of call to change. Fields left out stay.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateEventType {
    pub event_type_unid: Uuid,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    /// The call's length in minutes.
    pub duration_minutes: Option<i32>,
    pub buffer_minutes: Option<i32>,
    pub minimum_notice_hours: Option<i32>,
    pub booking_window_days: Option<i32>,
    pub questions: Option<Vec<Question>>,
    /// `false` switches the kind of call off without deleting it.
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WeeklyHours {
    /// The whole week: these rules replace every weekly rule. A weekday with
    /// no rule is closed.
    pub rules: Vec<AvailabilityRuleInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DateOverride {
    pub date: NaiveDate,
    /// The open hours that day, replacing the weekly hours. Empty: off all day.
    pub ranges: Vec<HoursRange>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DateRef {
    pub date: NaiveDate,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateProfile {
    /// The host's part of the booking link: `/book/<public_slug>`.
    pub public_slug: Option<String>,
    /// IANA timezone, such as `Europe/Berlin`.
    pub timezone: Option<String>,
    /// `false` pauses the booking page.
    pub is_active: Option<bool>,
    /// `#rrggbb`. Pro.
    pub accent_color: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Conferencing {
    /// Where calls happen: a Google Meet made with each calendar event, or
    /// the host's own link.
    pub app: ConferencingApp,
    /// The host's own meeting link. Needed for `custom_link`.
    pub meeting_link: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LeadRef {
    pub lead_unid: Uuid,
}

/// The MCP server one agent request is served by.
#[derive(Clone)]
pub struct CadenceMcp {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CadenceMcp {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List the host's calls, latest first. Filter by status and start time.",
        annotations(read_only_hint = true)
    )]
    async fn list_bookings(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<ListBookings>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        let found = async {
            let calls = bookings::list_filtered(
                &self.state.pool,
                host.unid,
                params.status,
                params.from,
                params.to,
                limit(params.limit).try_into().unwrap_or(i64::MAX),
            )
            .await?;
            let timezone = host_settings::timezone(&self.state.pool, host.unid).await?;
            calls
                .iter()
                .map(|call| local_call(call, &timezone))
                .collect::<AppResult<Vec<_>>>()
        };
        reply(found.await)
    }

    #[tool(
        description = "One call with the invitee's booking form answers.",
        annotations(read_only_hint = true)
    )]
    async fn get_booking(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<BookingRef>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        let found = async {
            let detail = bookings::detail(&self.state.pool, host.unid, params.booking_unid).await?;
            let timezone = host_settings::timezone(&self.state.pool, host.unid).await?;
            Ok(json!({
                "booking": local_call(&detail.booking, &timezone)?,
                "lead": detail.lead,
            }))
        };
        reply(found.await)
    }

    #[tool(
        description = "Cancel a call that has not started. The invitee is emailed.",
        annotations(destructive_hint = true, idempotent_hint = false)
    )]
    async fn cancel_booking(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<BookingRef>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        reply(
            bookings::cancel(
                &self.state.pool,
                &self.state.calendar,
                host.unid,
                params.booking_unid,
            )
            .await,
        )
    }

    #[tool(
        description = "Mark whether the invitee came to a call that is over.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn mark_no_show(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<NoShow>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        let request = NoShowRequest {
            no_show: params.no_show,
        };
        reply(
            bookings::set_no_show(&self.state.pool, host.unid, params.booking_unid, &request).await,
        )
    }

    #[tool(
        description = "Move a call that has not started to another open time. Both sides are emailed.",
        annotations(destructive_hint = false, idempotent_hint = false)
    )]
    async fn reschedule_booking(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<Reschedule>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        let request = RescheduleRequest {
            slot_start: params.slot_start,
        };
        reply(
            bookings::reschedule_by_host(
                &self.state.pool,
                &self.state.calendar,
                host.unid,
                params.booking_unid,
                request,
            )
            .await,
        )
    }

    #[tool(
        description = "Open times a kind of call can be booked or moved to, earliest first.",
        annotations(read_only_hint = true)
    )]
    async fn find_open_slots(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<FindSlots>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        let found = async {
            let page = self.bookable_page(host, params.event_type_unid).await?;
            let slots = bookings::public_slots(&self.state.pool, &self.state.calendar, &page)
                .await?
                .into_iter()
                .filter(|slot| params.from.is_none_or(|from| slot.start >= from))
                .filter(|slot| params.to.is_none_or(|to| slot.start < to))
                .take(limit(params.limit))
                .map(|slot| with_local(&slot, "start_local", slot.start, &page.timezone))
                .collect::<AppResult<Vec<_>>>()?;
            Ok(slots)
        };
        reply(found.await)
    }

    #[tool(
        description = "Book a call for an invitee at an open time. The invitee is emailed as if they had booked.",
        annotations(destructive_hint = false, idempotent_hint = false)
    )]
    async fn create_booking(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<CreateBooking>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        let created = async {
            let page = self.bookable_page(host, params.event_type_unid).await?;
            let request = CreateBookingRequest {
                lead_unid: None,
                slot_start: params.slot_start,
                timezone: params.timezone.unwrap_or_else(|| page.timezone.clone()),
                invitee_name: params.invitee_name,
                invitee_email: params.invitee_email,
            };
            bookings::create(&self.state.pool, &self.state.calendar, &page, request)
                .await
                .map(|created| created.booking)
        };
        reply(created.await)
    }

    #[tool(
        description = "The host's last 7 days in numbers. Business plan.",
        annotations(read_only_hint = true)
    )]
    async fn get_insights(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        reply(bookings::insights(&self.state.pool, host.unid).await)
    }

    #[tool(
        description = "The kinds of call the host offers.",
        annotations(read_only_hint = true)
    )]
    async fn list_event_types(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        reply(event_types::list(&self.state.pool, host.unid).await)
    }

    #[tool(
        description = "Add a kind of call.",
        annotations(destructive_hint = false, idempotent_hint = false)
    )]
    async fn create_event_type(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<EventTypeInput>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        reply(event_types::create(&self.state.pool, host.unid, params).await)
    }

    #[tool(
        description = "Change a kind of call. Fields left out stay.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn update_event_type(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<UpdateEventType>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        let saved = async {
            let current = event_types::list(&self.state.pool, host.unid)
                .await?
                .into_iter()
                .find(|event_type| event_type.unid == params.event_type_unid)
                .ok_or(AppError::NotFound)?;
            let input = EventTypeInput {
                slug: params.slug.unwrap_or(current.slug),
                title: params.title.unwrap_or(current.title),
                description: params.description.unwrap_or(current.description),
                duration_minutes: params.duration_minutes.or(current.duration_minutes),
                buffer_minutes: params.buffer_minutes.unwrap_or(current.buffer_minutes),
                minimum_notice_hours: params
                    .minimum_notice_hours
                    .unwrap_or(current.minimum_notice_hours),
                booking_window_days: params
                    .booking_window_days
                    .unwrap_or(current.booking_window_days),
                questions: params.questions.unwrap_or(current.questions),
                is_active: params.is_active.unwrap_or(current.is_active),
            };
            event_types::update(&self.state.pool, host.unid, params.event_type_unid, input).await
        };
        reply(saved.await)
    }

    #[tool(
        description = "Delete a kind of call. Calls already booked stay.",
        annotations(destructive_hint = true, idempotent_hint = false)
    )]
    async fn delete_event_type(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<EventTypeRef>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        reply(
            event_types::delete(&self.state.pool, host.unid, params.event_type_unid)
                .await
                .map(|()| json!({ "deleted": params.event_type_unid })),
        )
    }

    #[tool(
        description = "The host's weekly hours and the dates with their own hours, in the host's timezone.",
        annotations(read_only_hint = true)
    )]
    async fn get_availability(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        let found = async {
            let weekly = availability::list(&self.state.pool, host.unid).await?;
            let overrides = availability::list_overrides(&self.state.pool, host.unid).await?;
            Ok(json!({ "weekly": weekly, "date_overrides": overrides }))
        };
        reply(found.await)
    }

    #[tool(
        description = "Replace the host's whole weekly hours.",
        annotations(destructive_hint = true, idempotent_hint = true)
    )]
    async fn set_weekly_hours(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<WeeklyHours>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        let saved = async {
            availability::replace_weekly(&self.state.pool, host.unid, params.rules).await?;
            availability::list(&self.state.pool, host.unid).await
        };
        reply(saved.await)
    }

    #[tool(
        description = "Give one date its own hours, or take it off with no ranges.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn set_date_override(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<DateOverride>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        reply(
            availability::save_override(&self.state.pool, host.unid, params.date, params.ranges)
                .await,
        )
    }

    #[tool(
        description = "Give a date back its weekly hours.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn clear_date_override(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<DateRef>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        reply(
            availability::delete_override(&self.state.pool, host.unid, params.date)
                .await
                .map(|()| json!({ "cleared": params.date })),
        )
    }

    #[tool(
        description = "The host's booking page settings, timezone and booking link.",
        annotations(read_only_hint = true)
    )]
    async fn get_profile(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        reply(self.profile(host).await)
    }

    #[tool(
        description = "Change the host's booking link, timezone, accent colour, or pause the page. Fields left out stay.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn update_profile(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<UpdateProfile>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        let saved = async {
            let mut settings = host_settings::get_or_create(&self.state.pool, host).await?;
            if let Some(slug) = params.public_slug {
                settings.public_slug = slug;
            }
            if let Some(timezone) = params.timezone {
                settings.timezone = timezone;
            }
            if let Some(is_active) = params.is_active {
                settings.is_active = is_active;
            }
            if let Some(accent_color) = params.accent_color {
                settings.accent_color = accent_color;
            }
            host_settings::update(&self.state.pool, host.unid, settings).await?;
            self.profile(host).await
        };
        reply(saved.await)
    }

    #[tool(
        description = "Pick where the host's calls happen: Google Meet, or their own meeting link.",
        annotations(destructive_hint = false, idempotent_hint = true)
    )]
    async fn set_conferencing(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<Conferencing>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        let update = ConferencingUpdate {
            app: params.app,
            meeting_link: params.meeting_link,
        };
        reply(host_settings::set_conferencing(&self.state.pool, host, update).await)
    }

    #[tool(
        description = "People who started booking in the last days and left. Business plan.",
        annotations(read_only_hint = true)
    )]
    async fn list_unfinished_leads(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        reply(leads::unfinished(&self.state.pool, host.unid).await)
    }

    #[tool(
        description = "Email one person who left the booking form the host's booking link, once. Business plan.",
        annotations(destructive_hint = false, idempotent_hint = false)
    )]
    async fn follow_up_lead(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(params): Parameters<LeadRef>,
    ) -> Result<CallToolResult, ErrorData> {
        let host = host(&parts)?;
        if let Some(refused) = read_only(&parts) {
            return Ok(refused);
        }
        reply(
            leads::follow_up(&self.state.pool, host, params.lead_unid)
                .await
                .map(|()| json!({ "followed_up": params.lead_unid })),
        )
    }
}

impl CadenceMcp {
    /// A kind of call of the host that can be booked right now.
    async fn bookable_page(
        &self,
        host: &UserRecord,
        event_type_unid: Uuid,
    ) -> AppResult<BookingPage> {
        let settings = host_settings::get_or_create(&self.state.pool, host).await?;
        let event_type = event_types::list(&self.state.pool, host.unid)
            .await?
            .into_iter()
            .find(|event_type| event_type.unid == event_type_unid)
            .ok_or(AppError::NotFound)?;
        if !settings.is_active || !event_type.bookable {
            return Err(AppError::Conflict(
                "this kind of call is paused or past the host's plan, so it cannot be booked"
                    .to_owned(),
            ));
        }
        host_settings::bookable_page(&self.state.pool, &settings.public_slug, &event_type.slug)
            .await
    }

    async fn profile(&self, host: &UserRecord) -> AppResult<Value> {
        let settings = host_settings::get_or_create(&self.state.pool, host).await?;
        let booking_url = format!("{}/book/{}", app_url(), settings.public_slug);
        Ok(json!({
            "name": host.full_name,
            "email": host.email,
            "settings": settings,
            "booking_url": booking_url,
        }))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CadenceMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cadence", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    /// A read-only token sees only the tools that look, so its agent never
    /// plans a change it would be refused. The list differs per token, so it
    /// carries no shared cache hint.
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let read_only = context
            .extensions
            .get::<Parts>()
            .and_then(|parts| parts.extensions.get::<McpHost>())
            .is_some_and(|host| host.scope == AccessTokenScope::Read);
        let tools = self
            .tool_router
            .list_all()
            .into_iter()
            .filter(|tool| !read_only || only_looks(tool))
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }
}

fn only_looks(tool: &Tool) -> bool {
    tool.annotations
        .as_ref()
        .and_then(|annotations| annotations.read_only_hint)
        .unwrap_or(false)
}

fn host(parts: &Parts) -> Result<&UserRecord, ErrorData> {
    parts
        .extensions
        .get::<McpHost>()
        .map(|host| &host.user)
        .ok_or_else(|| ErrorData::internal_error("no host on the request", None))
}

/// The tool error a read-only token gets from a tool that changes things.
fn read_only(parts: &Parts) -> Option<CallToolResult> {
    let scope = parts.extensions.get::<McpHost>()?.scope;
    (scope == AccessTokenScope::Read).then(|| {
        CallToolResult::error(vec![ContentBlock::text(
            "this token is read-only; the host can make a full access token in Apps",
        )])
    })
}

/// A call with its start as the host's clock shows it.
fn local_call(call: &Booking, timezone: &str) -> AppResult<Value> {
    with_local(call, "slot_start_local", call.slot_start, timezone)
}

/// `value` as JSON, with `instant` on the host's clock added under `key`.
fn with_local<T: Serialize>(
    value: &T,
    key: &str,
    instant: DateTime<Utc>,
    timezone: &str,
) -> AppResult<Value> {
    let mut value =
        serde_json::to_value(value).map_err(|error| AppError::Internal(error.into()))?;
    if let Value::Object(fields) = &mut value {
        fields.insert(key.to_owned(), local_time(instant, timezone).into());
    }
    Ok(value)
}

fn local_time(instant: DateTime<Utc>, timezone: &str) -> Option<String> {
    availability::utc_to_local(instant, timezone)
        .map(|local| local.format(LOCAL_FORMAT).to_string())
}

fn limit(asked: Option<usize>) -> usize {
    asked.unwrap_or(LIST_DEFAULT).clamp(1, LIST_MAX)
}

/// A tool's answer as JSON. What the host could fix comes back as a tool
/// error the agent reads; a failure of ours stays a protocol error.
fn reply<T: Serialize>(result: AppResult<T>) -> Result<CallToolResult, ErrorData> {
    let message = match result {
        Ok(value) => {
            let value = serde_json::to_value(&value)
                .map_err(|error| ErrorData::internal_error(error.to_string(), None))?;
            let mut result = CallToolResult::success(vec![ContentBlock::text(value.to_string())]);
            // Structured content must be an object, so a list goes under `items`.
            result.structured_content = Some(if value.is_object() {
                value
            } else {
                json!({ "items": value })
            });
            return Ok(result);
        }
        Err(AppError::PlanRequired(feature)) => format!(
            "the host's plan does not include {feature}; they can upgrade at {}/dashboard/billing",
            app_url()
        ),
        Err(AppError::NotFound) => "not found, or not the host's".to_owned(),
        Err(error @ (AppError::Database(_) | AppError::Internal(_))) => {
            tracing::error!(?error, "mcp tool failed");
            return Err(ErrorData::internal_error("internal failure", None));
        }
        Err(error) => error.to_string(),
    };
    Ok(CallToolResult::error(vec![ContentBlock::text(message)]))
}
