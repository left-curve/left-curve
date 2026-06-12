import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { ReferralFaqs } from "../src/components/points/referral/ReferralFaqs";

function getByExactTextContent(text: string) {
  return screen.getByText((_content, element) => element?.textContent === text);
}

describe("ReferralFaqs", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders every localized referral FAQ and the commission example steps", () => {
    render(<ReferralFaqs />);

    expect(
      screen.getByRole("heading", { name: m["points.referral.faqs.title"]() }),
    ).toBeInTheDocument();

    for (const [index, question] of [
      m["points.referral.faqs.question1"](),
      m["points.referral.faqs.question2"](),
      m["points.referral.faqs.question3"](),
      m["points.referral.faqs.question4"](),
      m["points.referral.faqs.question5"](),
    ].entries()) {
      expect(screen.getByText(`${index + 1}. ${question}`)).toBeInTheDocument();
    }

    expect(screen.getByText(m["points.referral.faqs.answer1"]())).toBeInTheDocument();
    expect(screen.getByText(m["points.referral.faqs.answer2"]())).toBeInTheDocument();
    expect(screen.getByText(m["points.referral.faqs.answer3"]())).toBeInTheDocument();
    expect(screen.getByText(m["points.referral.faqs.answer4"]())).toBeInTheDocument();
    expect(getByExactTextContent(m["points.referral.faqs.answer5"]())).toBeInTheDocument();
    expect(screen.getByText(m["points.referral.faqs.answer5Example"]())).toBeInTheDocument();

    expect(screen.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      m["points.referral.faqs.answer5Step1"](),
      m["points.referral.faqs.answer5Step2"](),
      m["points.referral.faqs.answer5Step3"](),
      m["points.referral.faqs.answer5Step4"](),
      m["points.referral.faqs.answer5Step5"](),
      m["points.referral.faqs.answer5Step6"](),
    ]);
  });
});
