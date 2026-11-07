import { useEffect, useState } from "react";

/** Current Unix time in whole seconds. */
export function nowSecs(): number {
  return Math.floor(Date.now() / 1000);
}

/** Current Unix time in seconds, updated every second. */
export function useNow(): number {
  const [now, setNow] = useState(nowSecs);
  useEffect(() => {
    const timer = setInterval(() => setNow(nowSecs()), 1000);
    return () => clearInterval(timer);
  }, []);
  return now;
}
