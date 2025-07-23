import { Button } from "@mantine/core";
import { IconEdit } from "@tabler/icons-react";
import { useRemails } from "../hooks/useRemails.ts";
import { RouteName } from "../routes.ts";
import { RouteParams } from "../router.ts";

export default function EditButton({ route, params }: { route: RouteName; params: RouteParams }) {
  const { navigate } = useRemails();

  return (
    <Button
      variant="subtle"
      onClick={() => {
        navigate(route, params);
      }}
    >
      <IconEdit />
    </Button>
  );
}
