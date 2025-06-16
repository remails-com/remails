import { useComputedColorScheme } from "@mantine/core";

import logoBlack from "../img/remails-logo-black.svg";
import logoWhite from "../img/remails-logo-white.svg";

export function RemailsLogo({ style }: { style?: React.CSSProperties }) {
  const computedColorScheme = useComputedColorScheme();
  style = style ?? { height: 40 };

  return <img src={computedColorScheme == "light" ? logoBlack : logoWhite} alt="Remails logo" style={style} />;
}
