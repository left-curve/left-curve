import { useEffect, useState } from "react";
import { shallowEqual } from "@left-curve/utils";

type UseCountdownParameters = {
  date?: number | string | Date;
  showLeadingZeros?: boolean;
};

type TimeLeft = {
  days: string;
  hours: string;
  minutes: string;
  seconds: string;
};

const EMPTY_TIME_LEFT: TimeLeft = { days: "-", hours: "-", minutes: "-", seconds: "-" };
const EXPIRED_TIME_LEFT: TimeLeft = { days: "0", hours: "0", minutes: "0", seconds: "0" };
const EXPIRED_TIME_LEFT_PADDED: TimeLeft = {
  days: "00",
  hours: "00",
  minutes: "00",
  seconds: "00",
};

const getCountdownTargetTime = (date: UseCountdownParameters["date"]) => {
  if (!date) return null;
  const targetTime = +new Date(date);
  return Number.isFinite(targetTime) ? targetTime : null;
};

const getExpiredTimeLeft = (showLeadingZeros?: boolean) =>
  showLeadingZeros ? EXPIRED_TIME_LEFT_PADDED : EXPIRED_TIME_LEFT;

const calculateTimeLeft = (
  date: UseCountdownParameters["date"],
  showLeadingZeros?: boolean,
): TimeLeft => {
  const targetTime = getCountdownTargetTime(date);
  if (targetTime === null) return EMPTY_TIME_LEFT;

  const difference = targetTime - Date.now();
  if (difference <= 0) {
    return getExpiredTimeLeft(showLeadingZeros);
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

export function useCountdown(parameters: UseCountdownParameters) {
  const { date, showLeadingZeros } = parameters;

  const [timeLeft, setTimeLeft] = useState(() => calculateTimeLeft(date, showLeadingZeros));

  useEffect(() => {
    const updateTimeLeft = () => {
      const next = calculateTimeLeft(date, showLeadingZeros);
      setTimeLeft((previous) => (shallowEqual(previous, next) ? previous : next));
      return getCountdownTargetTime(date);
    };

    const targetTime = updateTimeLeft();
    if (targetTime === null || targetTime <= Date.now()) return;

    const timer = setInterval(() => {
      const nextTargetTime = updateTimeLeft();
      if (nextTargetTime === null || nextTargetTime <= Date.now()) clearInterval(timer);
    }, 1000);

    return () => clearInterval(timer);
  }, [date, showLeadingZeros]);

  return timeLeft;
}
