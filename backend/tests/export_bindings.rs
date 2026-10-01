use cadence_backend::domain::{
    auth::models::{LoginRequest, RegisterRequest, Role, SessionUser},
    availability::models::{AvailabilityRule, AvailabilityRuleInput, TimeSlot},
    bookings::models::{Booking, CreateBookingRequest},
    bug_reports::models::{AdminBugReport, AdminBugType},
    calling_visits::models::{CallingVisit, CallingVisitEngagement, CallingVisitRequest},
    leads::models::{Lead, LeadUpdate, SubmitLeadRequest},
    roleaccesses::models::RoleAccess,
};
use ts_rs::TS;

#[test]
fn export_bindings() {
    assert!(Role::export().is_ok());
    assert!(RegisterRequest::export().is_ok());
    assert!(LoginRequest::export().is_ok());
    assert!(SessionUser::export().is_ok());
    assert!(AvailabilityRuleInput::export().is_ok());
    assert!(AvailabilityRule::export().is_ok());
    assert!(TimeSlot::export().is_ok());
    assert!(CreateBookingRequest::export().is_ok());
    assert!(Booking::export().is_ok());
    assert!(AdminBugType::export().is_ok());
    assert!(AdminBugReport::export().is_ok());
    assert!(CallingVisitRequest::export().is_ok());
    assert!(CallingVisitEngagement::export().is_ok());
    assert!(CallingVisit::export().is_ok());
    assert!(LeadUpdate::export().is_ok());
    assert!(Lead::export().is_ok());
    assert!(SubmitLeadRequest::export().is_ok());
    assert!(RoleAccess::export().is_ok());
}
