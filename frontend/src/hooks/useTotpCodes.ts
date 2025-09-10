import { useRemails } from "./useRemails.ts";

export default function useTotpCodes() {
  const {
    state: { totpCodes },
  } = useRemails();

  return { totpCodes };
}
