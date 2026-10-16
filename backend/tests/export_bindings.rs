use cadence_backend::domain::{
    auth::models::{
        ChangePasswordRequest, ForgotPasswordRequest, LoginRequest, RegisterRequest,
        ResetPasswordRequest, Role, SessionUser, UpdateProfileRequest, UserStatus,
    },
    availability::models::{
        AvailabilityRule, AvailabilityRuleInput, DateOverride, DateOverrideInput, HoursRange,
        TimeSlot,
    },
    billing::{
        models::{AdminPlan, BillingSummary, CompPlan, RedirectUrl, StartCheckout},
        plans::{
            BillingInterval, Entitlements, Plan, PlanOffer, PlanPrice, Reminder, SubscriptionStatus,
        },
    },
    bookings::models::{
        Booking, BookingDetail, BookingStatus, CreateBookingRequest, CreatedBooking,
        ManagedBooking, RescheduleRequest,
    },
    bug_reports::models::{AddBugReportRequest, AdminBugReport, AdminBugType},
    calling_visits::models::{
        CallingVisit, CallingVisitEngagement, CallingVisitRequest, VisitFunnel,
    },
    emails::models::{AdminEmail, EmailState},
    event_types::models::{EventType, EventTypeInput},
    google_calendar::models::{
        CalendarEvent, EventDateTime, GoogleCalendarLoginRequest, GoogleOAuthConfigResponse,
    },
    host_settings::models::{
        AdminHost, BookingPage, ConferencingApp, ConferencingUpdate, EventTypeCard, HostPage,
        HostSettings,
    },
    leads::models::{Lead, LeadQualification, LeadUpdate, SubmitLeadRequest},
    roleaccesses::models::{AdminRoleAccess, RoleAccess},
};
use ts_rs::TS;

#[test]
fn export_bindings() {
    // One list, checked once: every wire type writes its TypeScript file.
    let exports = [
        ("Role", Role::export()),
        ("UserStatus", UserStatus::export()),
        ("RegisterRequest", RegisterRequest::export()),
        ("LoginRequest", LoginRequest::export()),
        ("SessionUser", SessionUser::export()),
        ("ForgotPasswordRequest", ForgotPasswordRequest::export()),
        ("ResetPasswordRequest", ResetPasswordRequest::export()),
        ("UpdateProfileRequest", UpdateProfileRequest::export()),
        ("ChangePasswordRequest", ChangePasswordRequest::export()),
        ("AvailabilityRuleInput", AvailabilityRuleInput::export()),
        ("AvailabilityRule", AvailabilityRule::export()),
        ("TimeSlot", TimeSlot::export()),
        ("HoursRange", HoursRange::export()),
        ("DateOverride", DateOverride::export()),
        ("DateOverrideInput", DateOverrideInput::export()),
        ("CalendarEvent", CalendarEvent::export()),
        ("EventDateTime", EventDateTime::export()),
        (
            "GoogleOAuthConfigResponse",
            GoogleOAuthConfigResponse::export(),
        ),
        (
            "GoogleCalendarLoginRequest",
            GoogleCalendarLoginRequest::export(),
        ),
        ("CreateBookingRequest", CreateBookingRequest::export()),
        ("BookingStatus", BookingStatus::export()),
        ("Booking", Booking::export()),
        ("BookingDetail", BookingDetail::export()),
        ("CreatedBooking", CreatedBooking::export()),
        ("ManagedBooking", ManagedBooking::export()),
        ("RescheduleRequest", RescheduleRequest::export()),
        ("AdminBugType", AdminBugType::export()),
        ("AddBugReportRequest", AddBugReportRequest::export()),
        ("AdminBugReport", AdminBugReport::export()),
        ("CallingVisitRequest", CallingVisitRequest::export()),
        ("CallingVisitEngagement", CallingVisitEngagement::export()),
        ("CallingVisit", CallingVisit::export()),
        ("LeadQualification", LeadQualification::export()),
        ("LeadUpdate", LeadUpdate::export()),
        ("Lead", Lead::export()),
        ("SubmitLeadRequest", SubmitLeadRequest::export()),
        ("RoleAccess", RoleAccess::export()),
        ("ConferencingApp", ConferencingApp::export()),
        ("ConferencingUpdate", ConferencingUpdate::export()),
        ("HostSettings", HostSettings::export()),
        ("BookingPage", BookingPage::export()),
        ("HostPage", HostPage::export()),
        ("EventTypeCard", EventTypeCard::export()),
        ("EventType", EventType::export()),
        ("EventTypeInput", EventTypeInput::export()),
        ("AdminHost", AdminHost::export()),
        ("AdminRoleAccess", AdminRoleAccess::export()),
        ("VisitFunnel", VisitFunnel::export()),
        ("EmailState", EmailState::export()),
        ("AdminEmail", AdminEmail::export()),
        ("Plan", Plan::export()),
        ("BillingInterval", BillingInterval::export()),
        ("SubscriptionStatus", SubscriptionStatus::export()),
        ("Reminder", Reminder::export()),
        ("Entitlements", Entitlements::export()),
        ("PlanPrice", PlanPrice::export()),
        ("PlanOffer", PlanOffer::export()),
        ("BillingSummary", BillingSummary::export()),
        ("StartCheckout", StartCheckout::export()),
        ("RedirectUrl", RedirectUrl::export()),
        ("AdminPlan", AdminPlan::export()),
        ("CompPlan", CompPlan::export()),
    ];
    for (name, export) in exports {
        assert!(export.is_ok(), "{name} did not export");
    }
}
