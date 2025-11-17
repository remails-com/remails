import { useApiKeys } from "../../hooks/useApiKeys.ts";
import { Loader } from "../../Loader.tsx";
import { Button, Flex, Table, Text } from "@mantine/core";
import { formatDateTime, KEY_ROLE_LABELS } from "../../util.ts";
import { IconExternalLink, IconPlus } from "@tabler/icons-react";
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

  let docs_url;
  if (window.location.hostname === "localhost") {
    docs_url = `http://${window.location.host}/docs/`;
  } else {
    docs_url = `https://docs.${window.location.host}`;
  }
  console.log(docs_url);

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
        <Button
          ms="xl"
          component="a"
          href={docs_url}
          target="_blank"
          variant="outline"
          rightSection={<IconExternalLink size={18} />}
        >
          API documentation
        </Button>
      </Flex>
    </>
  );
}
