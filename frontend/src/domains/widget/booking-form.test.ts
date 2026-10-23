import { describe, expect, it } from "vitest";

import {
  answerTo,
  EMPTY_FORM,
  type FormValues,
  formErrors,
  leadUpdate,
  queryValue,
  toggleValue,
} from "./booking-form";

const NOTHING_ASKED = [] as const;

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
    const errors = formErrors(VALID, [
      "currently_working_on",
      "how_heard_about_us",
      "urgency_level",
    ]);
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

describe("leadUpdate", () => {
  it("trims the answers and sends a blank one empty", () => {
    const update = leadUpdate(
      {
        ...VALID,
        firstName: "  Ana ",
        currentlyWorkingOn: "   ",
        howHeard: ["A friend", "Search"],
      },
      null,
    );
    expect(update.first_name).toBe("Ana");
    expect(answerTo(update.answers, "currently_working_on")).toBe("");
    expect(answerTo(update.answers, "how_heard_about_us")).toBe("A friend, Search");
    expect(update.country_code).toBe("33");
  });

  it("names the visit the form was opened in", () => {
    expect(leadUpdate(VALID, "visit-1").calling_visit_unid).toBe("visit-1");
  });
});

describe("queryValue", () => {
  it("reads one value of a query string", () => {
    expect(queryValue("?utm_source=ads&x=1", "utm_source")).toBe("ads");
    expect(queryValue("", "utm_source")).toBeNull();
  });
});
