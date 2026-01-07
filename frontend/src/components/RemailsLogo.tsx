import { useComputedColorScheme } from "@mantine/core";

import logoBlack from "/remails-logo-black.svg";
import logoWhite from "/remails-logo-white.svg";
import logoChristmasBlack from "/remails-logo-christmas-black.svg";
import logoChristmasWhite from "/remails-logo-christmas-white.svg";

const logos = {
  default: {
    light: logoBlack,
    dark: logoWhite,
  },
  december: {
    light: logoChristmasBlack,
    dark: logoChristmasWhite,
  },
};

export function RemailsLogo({ height }: { height?: number }) {
  const computedColorScheme = useComputedColorScheme();

  const is_december = new Date().getMonth() == 11;
  const logo = is_december ? logos.december : logos.default;

  height ??= 40;

  let transform = undefined;
  if (is_december) {
    height *= 1.045;
    transform = "translate(-8%, -3%)";
  }

  return (
    <img src={logo[computedColorScheme]} alt="Remails logo" style={{ height, verticalAlign: "bottom", transform }} />
  );
}
