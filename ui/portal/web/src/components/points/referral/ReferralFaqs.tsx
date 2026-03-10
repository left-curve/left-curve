import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";

export const ReferralFaqs: React.FC = () => {
  const faqs = [
    {
      question: m["points.referral.faqs.question1"](),
      answer: m["points.referral.faqs.answer1"](),
    },
    {
      question: m["points.referral.faqs.question2"](),
      answer: m["points.referral.faqs.answer2"](),
    },
    {
      question: m["points.referral.faqs.question3"](),
      answer: m["points.referral.faqs.answer3"](),
    },
  ];

  return (
    <div className="w-full flex flex-col gap-4">
      <h3 className="h4-bold text-ink-primary-900">{m["points.referral.faqs.title"]()}</h3>
      <div className="flex flex-col gap-4 bg-surface-primary-gray rounded-xl shadow-account-card p-4 lg:p-8">
        {faqs.map((faq, index) => (
          <div key={faq.question} className="flex flex-col gap-1">
            <p className="text-ink-primary-900 diatype-m-bold">
              {index + 1}. {faq.question}
            </p>
            <p className="text-ink-tertiary-500 diatype-m-regular">{faq.answer}</p>
          </div>
        ))}
      </div>
    </div>
  );
};
