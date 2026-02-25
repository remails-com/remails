import { useCredentials } from "../../hooks/useCredentials";
import { Loader } from "../../Loader.tsx";
import { Flex, Table, Text } from "@mantine/core";
import { formatDateTime } from "../../util.ts";
import { IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewCredential } from "./NewCredential.tsx";
import { SmtpInfo } from "./SmtpInfo.tsx";
import EditButton from "../EditButton.tsx";
import StyledTable from "../StyledTable.tsx";
import InfoAlert from "../InfoAlert.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";

export function CredentialsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { credentials } = useCredentials();

  if (credentials === null) {
    return <Loader />;
  }

  const rows = credentials.map((credential) => {
    const username_parts = credential.username.split("-");
    const username = (
      <>
        <Text span c="dimmed">
          {username_parts[0]}-
        </Text>
        <Text span>{username_parts.slice(1).join("-")}</Text>
      </>
    );

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
          <EditButton
            route="projects.project.credentials.credential"
            params={{
              credential_id: credential.id,
            }}
          />
        </Table.Td>
      </Table.Tr>
    );
  });

  return (
    <>
      <InfoAlert stateName={"smtp-cred"}>
        <Text mb="sm">
          Create SMTP credentials for this Project. Each set of credentials is unique to the Project and can be used to
          authenticate email sending. You can create multiple credentials per Project if needed.
        </Text>
        <SmtpInfo />
      </InfoAlert>
      <NewCredential opened={opened} close={close} />
      <StyledTable
        headers={[{ miw: "10rem", children: "Username" }, "Description", { miw: "10rem", children: "Updated" }, ""]}
      >
        {rows}
      </StyledTable>
      <Flex justify="center" my="md">
        <MaintainerButton onClick={() => open()} leftSection={<IconPlus />}>
          New credential
        </MaintainerButton>
      </Flex>
    </>
  );
}
