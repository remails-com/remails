import {Center, Loader as MantineLoader} from "@mantine/core";
import {useEffect, useState} from "react";
import {useOrganizations} from "./hooks/useOrganizations.ts";

export function Loader() {
  const [delayed, setDelayed] = useState(true);
  const {organizations} = useOrganizations();

  useEffect(() => {
    const timeout = setTimeout(() => setDelayed(false), 50);
    return () => clearTimeout(timeout);
  }, []);

  if (delayed) {
    return (
      <></>
    )
  }

  if (organizations?.length === 0) {
    // In this case, the user has to create an organization first.
    return <></>
  }

  return (
    <Center>
      <MantineLoader color="gray" size="xl" type="dots"/>
    </Center>
  );
}
