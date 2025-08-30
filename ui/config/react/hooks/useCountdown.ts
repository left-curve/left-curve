import { useEffect, useState } from "react";

type UseCountdownParameters = {
  date?: number | string | Date;
  showLeadingZeros?: boolean;
};

export function useCountdown(parameters: UseCountdownParameters) {
  const { date, showLeadingZeros } = parameters;
  const calculateTimeLeft = () => {
    if (!date) return { days: "-", hours: "-", minutes: "-", seconds: "-" };
    const difference = +new Date(date) - Date.now();
    if (difference <= 0) {
      return showLeadingZeros
        ? { days: "00", hours: "00", minutes: "00", seconds: "00" }
        : { days: "0", hours: "0", minutes: "0", seconds: "0" };
    }

    const days = Math.floor(difference / (1000 * 60 * 60 * 24)).toString();
    const hours = Math.floor((difference / (1000 * 60 * 60)) % 24).toString();
    const minutes = Math.floor((difference / (1000 * 60)) % 60).toString();
    const seconds = Math.floor((difference / 1000) % 60).toString();

    return {
      days: showLeadingZeros ? days.padStart(2, "0") : days,
      hours: showLeadingZeros ? hours.padStart(2, "0") : hours,
      minutes: showLeadingZeros ? minutes.padStart(2, "0") : minutes,
      seconds: showLeadingZeros ? seconds.padStart(2, "0") : seconds,
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
