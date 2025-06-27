import { Center, Loader as MantineLoader } from "@mantine/core";
import { useOrganizations } from "./hooks/useOrganizations.ts";

export function Loader() {
  const { organizations } = useOrganizations();

  if (organizations?.length === 0) {
    // In this case, the user has to create an organization first.
    return <></>;
  }

  return (
    <Center>
      <MantineLoader color="gray" size="xl" type="dots" />
    </Center>
  );
}
