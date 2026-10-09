use cadence_backend::domain::{
    auth::models::{LoginRequest, RegisterRequest, Role, SessionUser, UserStatus},
    availability::models::{AvailabilityRule, AvailabilityRuleInput, TimeSlot},
    bookings::models::{Booking, BookingDetail, BookingStatus, CreateBookingRequest},
    bug_reports::models::{AddBugReportRequest, AdminBugReport, AdminBugType},
    calling_visits::models::{
        CallingVisit, CallingVisitEngagement, CallingVisitRequest, VisitFunnel,
    },
    emails::models::{AdminEmail, EmailState},
    google_calendar::models::{CalendarEvent, EventDateTime},
    host_settings::models::{AdminHost, BookingPage, HostSettings},
    leads::models::{Lead, LeadQualification, LeadUpdate, SubmitLeadRequest},
    roleaccesses::models::{AdminRoleAccess, RoleAccess},
};
use ts_rs::TS;

#[test]
fn export_bindings() {
    assert!(Role::export().is_ok());
    assert!(UserStatus::export().is_ok());
    assert!(RegisterRequest::export().is_ok());
    assert!(LoginRequest::export().is_ok());
    assert!(SessionUser::export().is_ok());
    assert!(AvailabilityRuleInput::export().is_ok());
    assert!(AvailabilityRule::export().is_ok());
    assert!(TimeSlot::export().is_ok());
    assert!(CalendarEvent::export().is_ok());
    assert!(EventDateTime::export().is_ok());
    assert!(CreateBookingRequest::export().is_ok());
    assert!(BookingStatus::export().is_ok());
    assert!(Booking::export().is_ok());
    assert!(BookingDetail::export().is_ok());
    assert!(AdminBugType::export().is_ok());
    assert!(AddBugReportRequest::export().is_ok());
    assert!(AdminBugReport::export().is_ok());
    assert!(CallingVisitRequest::export().is_ok());
    assert!(CallingVisitEngagement::export().is_ok());
    assert!(CallingVisit::export().is_ok());
    assert!(LeadQualification::export().is_ok());
    assert!(LeadUpdate::export().is_ok());
    assert!(Lead::export().is_ok());
    assert!(SubmitLeadRequest::export().is_ok());
    assert!(RoleAccess::export().is_ok());
    assert!(HostSettings::export().is_ok());
    assert!(BookingPage::export().is_ok());
    assert!(AdminHost::export().is_ok());
    assert!(AdminRoleAccess::export().is_ok());
    assert!(VisitFunnel::export().is_ok());
    assert!(EmailState::export().is_ok());
    assert!(AdminEmail::export().is_ok());
}
