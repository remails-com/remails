import { Button } from "@mantine/core";
import { IconEdit } from "@tabler/icons-react";
import { useRemails } from "../hooks/useRemails.ts";
import { RouteName } from "../routes.ts";
import { RouteParams } from "../router.ts";

export default function EditButton({ route, params }: { route: RouteName; params: RouteParams }) {
  const { navigate, routeToPath } = useRemails();

  return (
    <Button
      variant="subtle"
      component="a"
      href={routeToPath(route, params)}
      onClick={(e) => {
        e.preventDefault();
        navigate(route, params);
      }}
    >
      <IconEdit />
    </Button>
  );
}
