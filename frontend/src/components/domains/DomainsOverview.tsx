import { useDomains } from "../../hooks/useDomains.ts";
import { Loader } from "../../Loader.tsx";
import { Button, Flex, Table } from "@mantine/core";
import { formatDateTime } from "../../util.ts";
import { IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewDomain } from "./NewDomain.tsx";
import { useProjects } from "../../hooks/useProjects.ts";
import { Link } from "../../Link.tsx";
import InfoAlert from "../InfoAlert.tsx";
import StyledTable from "../StyledTable.tsx";
import VerificationBadge from "./VerificationBadge.tsx";
import EditButton from "../EditButton.tsx";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";

export default function DomainsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { currentProject } = useProjects();
  const { domains } = useDomains();

  if (domains === null) {
    return <Loader />;
  }

  const route = currentProject ? "projects.project.domains.domain" : "domains.domain";

  const rows = domains.map((domain) => (
    <Table.Tr key={domain.id}>
      <Table.Td>
        <Link to={route} params={{ domain_id: domain.id }}>
          {domain.domain}
        </Link>
      </Table.Td>
      <Table.Td>
        <Link to={route} params={{ domain_id: domain.id }}>
          <VerificationBadge status={domain.verification_status} />
        </Link>
      </Table.Td>
      <Table.Td>{formatDateTime(domain.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <EditButton
          route={route}
          params={{
            domain_id: domain.id,
          }}
        />
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      {!currentProject && <OrganizationHeader />}
      {currentProject ? (
        <InfoAlert stateName="project-domains">
          Domains added here can be used by any Stream in this project. Each domain must be verified via DNS (SPF, DKIM,
          and DMARC) before emails can be sent from it.
        </InfoAlert>
      ) : (
        <InfoAlert stateName="global-domains">
          Organization domains are available across all projects in the organization. Use this to manage domains
          centrally if they’re shared between multiple projects.
        </InfoAlert>
      )}
      <NewDomain opened={opened} close={close} projectId={currentProject?.id || null} />
      <StyledTable headers={["Domains", "DNS Status", "Updated", ""]}>{rows}</StyledTable>
      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Domain
        </Button>
      </Flex>
    </>
  );
}
