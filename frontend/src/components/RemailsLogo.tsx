import { useComputedColorScheme } from "@mantine/core";

import logoBlack from "/remails-logo-black.svg";
import logoWhite from "/remails-logo-white.svg";
import React from "react";

export function RemailsLogo({ style }: { style?: React.CSSProperties }) {
  const computedColorScheme = useComputedColorScheme();
  style = style ?? { height: 40, verticalAlign: "bottom" };

  return <img src={computedColorScheme == "light" ? logoBlack : logoWhite} alt="Remails logo" style={style} />;
}
