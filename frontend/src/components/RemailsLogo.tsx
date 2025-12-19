import { useComputedColorScheme } from "@mantine/core";

import logoBlack from "/remails-logo-black.svg";
import logoWhite from "/remails-logo-white.svg";
import React from "react";

export function RemailsLogo({ style }: { style?: React.CSSProperties }) {
  const computedColorScheme = useComputedColorScheme();
  style = style ?? { height: 50, verticalAlign: "bottom", marginTop: "-5px", marginLeft: "-10px" };

  return <img src={computedColorScheme == "light" ? logoBlack : logoWhite} alt="Remails logo" style={style} />;
}
