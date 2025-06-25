import { useCredentials } from "../../hooks/useCredentials";
import { Loader } from "../../Loader.tsx";
import { useRemails } from "../../hooks/useRemails.ts";
import { Button, Flex, Table, Text } from "@mantine/core";
import { formatDateTime } from "../../util.ts";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewCredential } from "./NewCredential.tsx";

export function CredentialsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const {
    state: { loading },
    navigate,
  } = useRemails();
  const { credentials } = useCredentials();

  if (loading || credentials === null) {
    return <Loader />;
  }

  const rows = credentials.map((credential) => {
    const username_parts = credential.username.split("-", 2);
    let username = (
      <>
        <Text span c="dimmed">
          {username_parts[0]}-
        </Text>
        <Text span>{username_parts[1]}</Text>
      </>
    );
    if (username_parts.length === 1) {
      // Only relevant for testing credentials that do not have the organization ID prepended.
      // TODO remove this special case once all credentials have the organization ID prepended.
      username = <>{credential.username}</>;
    }
    return (
      <Table.Tr key={credential.id}>
        <Table.Td>{username}</Table.Td>
        <Table.Td>
          <Text size="sm" lineClamp={2}>
            {credential.description}
          </Text>
        </Table.Td>
        <Table.Td>{formatDateTime(credential.updated_at)}</Table.Td>
        <Table.Td align={"right"}>
          <Button
            variant="subtle"
            onClick={() =>
              navigate("projects.project.streams.stream.credentials.credential", {
                credential_id: credential.id,
              })
            }
          >
            <IconEdit />
          </Button>
        </Table.Td>
      </Table.Tr>
    );
  });

  return (
    <>
      <NewCredential opened={opened} close={close} />
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th miw="10rem">Username</Table.Th>
            <Table.Th>Description</Table.Th>
            <Table.Th miw="10rem">Updated</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Credential
        </Button>
      </Flex>
    </>
  );
}
