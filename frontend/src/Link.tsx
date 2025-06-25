import { RouteName, RouteParams } from "./router.ts";
import { useRemails } from "./hooks/useRemails.ts";
import { Anchor } from "@mantine/core";

interface LinkProps {
  to: RouteName,
  params?: RouteParams,
  query?: RouteParams,
  children: React.ReactNode;
}

export function Link({ to, params, query, children }: LinkProps) {
  const { navigate } = useRemails();

  const onClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    navigate(to, params, query);
  };

  return (
    <Anchor onClick={onClick} underline="always">
      {children}
    </Anchor>
  );
}
