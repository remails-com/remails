import { useDomains } from "../../hooks/useDomains.ts";
import { Loader } from "../../Loader.tsx";
import { Flex, Group, Pagination, Stack, Table, Text } from "@mantine/core";
import { formatDateTime } from "../../util.ts";
import { IconPlus, IconServer } from "@tabler/icons-react";
import { useDisclosure, useScrollIntoView } from "@mantine/hooks";
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
import { useState } from "react";

const PER_PAGE = 20;

function DomainRow({ domain }: { domain: Domain }) {
  const project_name = useProjectWithId(domain.project_id)?.name;

  return (
    <Table.Tr>
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
        {domain.project_id ? (
          <Link to={"projects.project"} params={{ proj_id: domain.project_id }}>
            <Group gap="0.4em">
              <IconServer /> {project_name ?? domain.project_id}
            </Group>
          </Link>
        ) : (
          <Text fs="italic" c="dimmed">
            any project
          </Text>
        )}
      </Table.Td>
      <Table.Td>{formatDateTime(domain.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <EditButton
          route={"domains.domain.settings"}
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
  const [activePage, setPage] = useState(1);

  const { scrollIntoView, targetRef } = useScrollIntoView<HTMLTableSectionElement>({
    duration: 500,
    offset: 100,
  });

  if (domains === null) {
    return <Loader />;
  }

  return (
    <>
      <OrganizationHeader />
      <InfoAlert stateName="project-domains">
        Domains must be verified via DNS (SPF, DKIM, and DMARC) before emails can be sent from them. Optionally, domains
        can be restricted to a single project.
      </InfoAlert>

      <NewDomain opened={opened} close={close} />
      <StyledTable ref={targetRef} headers={["Domains", "DNS Status", "Usable by", "Updated", ""]}>
        {domains.slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE).map((domain) => (
          <DomainRow domain={domain} key={domain.id} />
        ))}
      </StyledTable>
      <Flex justify="center" mt="md">
        <Stack>
          {domains.length > PER_PAGE && (
            <Pagination
              value={activePage}
              onChange={(p) => {
                setPage(p);
                scrollIntoView({ alignment: "start" });
              }}
              total={Math.ceil(domains.length / PER_PAGE)}
            />
          )}
          <MaintainerButton onClick={() => open()} leftSection={<IconPlus />}>
            New Domain
          </MaintainerButton>
        </Stack>
      </Flex>
    </>
  );
}
