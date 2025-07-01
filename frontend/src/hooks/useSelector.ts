import { useContext } from "react";
import { RemailsContext } from "./useRemails";
import { State } from "../types";

export default function useSelector<T>(selector: (state: State) => T): NonNullable<T> {
  const { state } = useContext(RemailsContext);
  const selectedState = selector(state);

  if (selectedState === null || selectedState === undefined) {
    throw new Error("Selector returned undefined. Ensure the selector is correct and returns a valid value.");
  }

  return selectedState;
}
