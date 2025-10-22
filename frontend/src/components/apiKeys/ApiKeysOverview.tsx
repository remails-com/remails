import { useApiKeys } from "../../hooks/useApiKeys.ts";
import { Loader } from "../../Loader.tsx";
import { Flex, Table, Text } from "@mantine/core";
import { formatDateTime, KEY_ROLE_LABELS } from "../../util.ts";
import { IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewApiKey } from "./NewApiKey.tsx";
import EditButton from "../EditButton.tsx";
import StyledTable from "../StyledTable.tsx";
import InfoAlert from "../InfoAlert.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";

export default function ApiKeysOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { apiKeys } = useApiKeys();

  if (apiKeys === null) {
    return <Loader />;
  }

  const rows = apiKeys.map((api_key) => {
    return (
      <Table.Tr key={api_key.id}>
        <Table.Td>{api_key.id}</Table.Td>
        <Table.Td>
          <Text size="sm" lineClamp={2}>
            {api_key.description}
          </Text>
        </Table.Td>
        <Table.Td>{KEY_ROLE_LABELS[api_key.role]}</Table.Td>
        <Table.Td>{formatDateTime(api_key.updated_at)}</Table.Td>
        <Table.Td align={"right"}>
          <EditButton
            route="settings.API keys.API key"
            params={{
              api_key_id: api_key.id,
            }}
          />
        </Table.Td>
      </Table.Tr>
    );
  });

  return (
    <>
      <InfoAlert stateName={"api-keys"}>
        Create API keys for this organization. API keys can be used to automate actions within this organization, such
        as viewing email statuses, managing streams, and tracking your quota.
      </InfoAlert>
      <NewApiKey opened={opened} close={close} />
      <StyledTable
        headers={[
          { miw: "10rem", children: "ID" },
          "Description",
          "Access level",
          { miw: "10rem", children: "Updated" },
          "",
        ]}
      >
        {rows}
      </StyledTable>
      <Flex justify="center" my="md">
        <MaintainerButton onClick={() => open()} leftSection={<IconPlus />}>
          New API Key
        </MaintainerButton>
      </Flex>
    </>
  );
}
