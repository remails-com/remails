import { useSelector } from "./useSelector.ts";

export function useApiUsers() {
  const apiUsers = useSelector((state) => state.apiUsers);

  return { apiUsers };
}
