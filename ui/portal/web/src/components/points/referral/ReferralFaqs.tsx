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
    {
      question: m["points.referral.faqs.question4"](),
      answer: m["points.referral.faqs.answer4"](),
    },
  ];

  const answer5Steps = [
    m["points.referral.faqs.answer5Step1"](),
    m["points.referral.faqs.answer5Step2"](),
    m["points.referral.faqs.answer5Step3"](),
    m["points.referral.faqs.answer5Step4"](),
    m["points.referral.faqs.answer5Step5"](),
    m["points.referral.faqs.answer5Step6"](),
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
            <p className="text-ink-tertiary-500 diatype-m-regular whitespace-pre-line">{faq.answer}</p>
          </div>
        ))}
        <div className="flex flex-col gap-1">
          <p className="text-ink-primary-900 diatype-m-bold">
            {faqs.length + 1}. {m["points.referral.faqs.question5"]()}
          </p>
          <p className="text-ink-tertiary-500 diatype-m-regular whitespace-pre-line">
            {m["points.referral.faqs.answer5"]()}
          </p>
          <p className="text-ink-tertiary-500 diatype-m-bold mt-2">
            {m["points.referral.faqs.answer5Example"]()}
          </p>
          <ul className="list-disc list-inside text-ink-tertiary-500 diatype-m-regular flex flex-col gap-1">
            {answer5Steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
};
