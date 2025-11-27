import { useDomains } from "../../hooks/useDomains.ts";
import { Loader } from "../../Loader.tsx";
import { Flex, Table } from "@mantine/core";
import { formatDateTime } from "../../util.ts";
import { IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewDomain } from "./NewDomain.tsx";
import { Link } from "../../Link.tsx";
import InfoAlert from "../InfoAlert.tsx";
import StyledTable from "../StyledTable.tsx";
import VerificationBadge from "./VerificationBadge.tsx";
import EditButton from "../EditButton.tsx";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";
import { useProjectWithId } from "../../hooks/useProjects.ts";
import { Domain } from "../../types.ts";

function DomainRow({ domain }: { domain: Domain }) {
  const project_name = useProjectWithId(domain.project_id)?.name;

  return (
    <Table.Tr key={domain.id}>
      <Table.Td>
        <Link to={"domains.domain"} params={{ domain_id: domain.id }}>
          {domain.domain}
        </Link>
      </Table.Td>
      <Table.Td>
        <Link to={"domains.domain"} params={{ domain_id: domain.id }}>
          <VerificationBadge status={domain.verification_status} />
        </Link>
      </Table.Td>
      <Table.Td>
        {domain.project_id && (
          <Link to={"projects.project"} params={{ proj_id: domain.project_id }}>
            {project_name ?? domain.project_id}
          </Link>
        )}
      </Table.Td>
      <Table.Td>{formatDateTime(domain.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <EditButton
          route={"domains.domain"}
          params={{
            domain_id: domain.id,
          }}
        />
      </Table.Td>
    </Table.Tr>
  );
}

export default function DomainsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { domains } = useDomains();

  if (domains === null) {
    return <Loader />;
  }

  return (
    <>
      <OrganizationHeader />
      <InfoAlert stateName="project-domains">
        Domains must be verified via DNS (SPF, DKIM, and DMARC) before emails can be sent from it. Optionally, domains
        can be restricted to a single project.
      </InfoAlert>

      <NewDomain opened={opened} close={close} />
      <StyledTable headers={["Domains", "DNS Status", "Project", "Updated", ""]}>
        {domains.map((domain) => (
          <DomainRow domain={domain} />
        ))}
      </StyledTable>
      <Flex justify="center" mt="md">
        <MaintainerButton onClick={() => open()} leftSection={<IconPlus />}>
          New Domain
        </MaintainerButton>
      </Flex>
    </>
  );
}
