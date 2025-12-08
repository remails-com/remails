import { useSelector } from "./useSelector.ts";

export function useRuntimeConfig() {
  const runtimeConfig = useSelector((state) => state.runtimeConfig);

  return { runtimeConfig };
}
