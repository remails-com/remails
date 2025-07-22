import { useComputedColorScheme } from "@mantine/core";

import logoBlack from "../img/remails-logo-black.svg";
import logoWhite from "../img/remails-logo-white.svg";
import { Link } from "../Link.tsx";
import React from "react";

export function RemailsLogo({ style }: { style?: React.CSSProperties }) {
  const computedColorScheme = useComputedColorScheme();
  style = style ?? { height: 40 };

  return (
    <Link to="projects">
      <img src={computedColorScheme == "light" ? logoBlack : logoWhite} alt="Remails logo" style={style} />
    </Link>
  );
}
