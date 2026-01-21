import { Anchor, AnchorProps } from "@mantine/core";
import { useRemails } from "./hooks/useRemails.ts";
import { RouteParams } from "./router.ts";
import { RouteName } from "./routes.ts";

interface LinkProps {
  to: RouteName;
  params?: RouteParams;
  underline?: "always" | "hover" | "never";
  children: React.ReactNode;
  style?: AnchorProps;
}

export function Link({ to, params, underline, children, style }: LinkProps) {
  const { navigate, getRoute } = useRemails();

  const onClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
    if (e.defaultPrevented || e.ctrlKey || e.metaKey) {
      return;
    }

    e.preventDefault();
    navigate(to, params);
  };

  const route = getRoute(to ?? "default", params);

  return (
    <Anchor href={route.fullPath} onClick={onClick} underline={underline || "always"} {...style}>
      {children}
    </Anchor>
  );
}
