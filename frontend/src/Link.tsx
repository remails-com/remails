import { RouteName, RouteParams } from "./router.ts";
import { useRemails } from "./hooks/useRemails.ts";
import { Anchor } from "@mantine/core";

interface LinkProps {
  to: RouteName;
  params?: RouteParams;
  underline?: "always" | "hover" | "never";
  children: React.ReactNode;
}

export function Link({ to, params, underline, children }: LinkProps) {
  const { navigate } = useRemails();

  const onClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    navigate(to, params);
  };

  return (
    <Anchor onClick={onClick} underline={underline || "always"}>
      {children}
    </Anchor>
  );
}
