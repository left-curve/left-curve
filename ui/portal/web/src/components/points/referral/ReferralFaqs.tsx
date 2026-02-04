import type React from "react";

type FaqItem = {
  question: string;
  answer: string;
};

const faqs: FaqItem[] = [
  {
    question: "What is an active friend?",
    answer: "A friend with a contract trading volume >= 100K will become your valid friend.",
  },
  {
    question: "How can I upgrade my tier, and when do the benefits take effect?",
    answer: "A friend with a contract trading volume >= 100K will become your valid friend.",
  },
  {
    question: "How long is my tier valid?",
    answer: "A friend with a contract trading volume >= 100K will become your valid friend.",
  },
];

export const ReferralFaqs: React.FC = () => {
  return (
    <div className="w-full flex flex-col gap-4">
      <h3 className="h4-bold text-ink-primary-900">FAQs</h3>
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
