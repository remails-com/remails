import {Center, Loader as MantineLoader} from "@mantine/core";
import {useEffect, useState} from "react";

export function Loader() {
  const [delayed, setDelayed] = useState(true);

  useEffect(() => {
    const timeout = setTimeout(() => setDelayed(false), 50);
    return () => clearTimeout(timeout);
  }, []);

  if (delayed) {
    return (
      <></>
    )
  }

  return (
    <Center>
      <MantineLoader color="gray" size="xl" type="dots"/>
    </Center>
  );
}
