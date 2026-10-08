import { describe, expect, it } from "vitest";

import { EMPTY_FORM, type FormValues, formErrors, toggleValue } from "./booking-form";

const NOTHING_ASKED = {
  ask_currently_working_on: false,
  ask_how_heard_about_us: false,
  ask_what_stopping_you: false,
  ask_investment_comfort: false,
  ask_urgency_level: false,
};

const VALID: FormValues = {
  ...EMPTY_FORM,
  countryVariant: "France",
  phone: "612345678",
  firstName: "Ana",
  lastName: "Kovac",
  email: "ana@example.com",
};

describe("formErrors", () => {
  it("passes a complete form", () => {
    expect(formErrors(VALID, NOTHING_ASKED)).toEqual({});
  });

  it("names every missing required field", () => {
    expect(Object.keys(formErrors(EMPTY_FORM, NOTHING_ASKED)).sort()).toEqual([
      "email",
      "firstName",
      "lastName",
      "phone",
    ]);
  });

  it("rejects a name with digits and a malformed email", () => {
    const errors = formErrors(
      { ...VALID, firstName: "An4", email: "ana@example" },
      NOTHING_ASKED,
    );
    expect(Object.keys(errors).sort()).toEqual(["email", "firstName"]);
  });

  it("rejects a short phone number and one digit repeated", () => {
    expect(
      formErrors({ ...VALID, phone: "61234567" }, NOTHING_ASKED).phone,
    ).toBeDefined();
    expect(
      formErrors({ ...VALID, phone: "611111178" }, NOTHING_ASKED).phone,
    ).toBeDefined();
  });

  it("requires an answer only to the choice questions a host asks", () => {
    const errors = formErrors(VALID, {
      ...NOTHING_ASKED,
      ask_currently_working_on: true,
      ask_how_heard_about_us: true,
      ask_urgency_level: true,
    });
    expect(errors).toEqual({
      howHeard: "Pick at least one option.",
      urgencyLevel: "Pick one option.",
    });
  });
});

describe("toggleValue", () => {
  it("adds a value that is missing and removes one that is there", () => {
    expect(toggleValue(["Google"], "LinkedIn")).toEqual(["Google", "LinkedIn"]);
    expect(toggleValue(["Google", "LinkedIn"], "Google")).toEqual(["LinkedIn"]);
  });
});
