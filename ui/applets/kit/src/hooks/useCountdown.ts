import { useEffect, useState } from "react";

type UseCountdownParameters = {
  date?: number | string | Date;
  withPad?: boolean;
};

export function useCountdown(parameters: UseCountdownParameters) {
  const { date, withPad } = parameters;
  const calculateTimeLeft = () => {
    if (!date) return { days: "-", hours: "-", minutes: "-", seconds: "-" };
    const difference = +new Date(date) - +new Date();
    if (difference <= 0) {
      return { days: 0, hours: 0, minutes: 0, seconds: 0 };
    }

    const days = Math.floor(difference / (1000 * 60 * 60 * 24)).toString();
    const hours = Math.floor((difference / (1000 * 60 * 60)) % 24).toString();
    const minutes = Math.floor((difference / (1000 * 60)) % 60).toString();
    const seconds = Math.floor((difference / 1000) % 60).toString();

    return {
      days: withPad ? days.padStart(2, "0") : days,
      hours: withPad ? hours.padStart(2, "0") : hours,
      minutes: withPad ? minutes.padStart(2, "0") : minutes,
      seconds: withPad ? seconds.padStart(2, "0") : seconds,
    };
  };

  const [timeLeft, setTimeLeft] = useState(calculateTimeLeft);

  useEffect(() => {
    const timer = setInterval(() => {
      setTimeLeft(calculateTimeLeft());
    }, 1000);

    return () => clearInterval(timer);
  }, [date]);

  return timeLeft;
}
