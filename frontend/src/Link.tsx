import { RouteName, RouteParams } from "./router.ts";
import { useRemails } from "./hooks/useRemails.ts";
import { Anchor } from "@mantine/core";

interface LinkProps {
  to: RouteName;
  params?: RouteParams;
  children: React.ReactNode;
}

export function Link({ to, params, children }: LinkProps) {
  const { navigate } = useRemails();

  const onClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    navigate(to, params);
  };

  return (
    <Anchor onClick={onClick} underline="always">
      {children}
    </Anchor>
  );
}
