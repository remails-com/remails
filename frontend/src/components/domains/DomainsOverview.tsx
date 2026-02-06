import { useDomains } from "../../hooks/useDomains.ts";
import { Loader } from "../../Loader.tsx";
import { Flex, Pagination, Stack, Table, Text } from "@mantine/core";
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
import { Domain } from "../../types.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { useState } from "react";
import SearchInput from "../SearchInput.tsx";
import ProjectLink from "../ProjectLink.tsx";

const PER_PAGE = 20;
const SHOW_SEARCH = 10;

function DomainRow({ domain }: { domain: Domain }) {
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
          <ProjectLink project_id={domain.project_id} />
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
  const {
    state: { routerState },
    navigate,
  } = useRemails();
  const [opened, { open, close }] = useDisclosure(false);
  const { domains } = useDomains();
  const [searchQuery, setSearchQuery] = useState(routerState.params.q || "");

  if (domains === null) {
    return <Loader />;
  }

  const filteredDomains =
    searchQuery.length == 0
      ? domains
      : domains.filter((domain) => domain.domain.toLowerCase().includes(searchQuery.toLowerCase()));

  const totalPages = Math.ceil(filteredDomains.length / PER_PAGE);
  const activePage = Math.min(Math.max(parseInt(routerState.params.p) || 1, 1), totalPages);

  const rows = filteredDomains
    .slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE)
    .map((domain) => <DomainRow domain={domain} key={domain.id} />);

  return (
    <>
      <OrganizationHeader />

      <InfoAlert stateName="project-domains">
        Domains must be verified via DNS (SPF, DKIM, and DMARC) before emails can be sent from them. Optionally, domains
        can be restricted to a single project.
      </InfoAlert>

      <NewDomain opened={opened} close={close} />

      {(domains.length > SHOW_SEARCH || searchQuery.length > 0) && (
        <SearchInput searchQuery={searchQuery} setSearchQuery={setSearchQuery} />
      )}

      {searchQuery.length > 0 && filteredDomains.length == 0 && (
        <Text fs="italic" c="gray">
          No domains found...
        </Text>
      )}

      <StyledTable headers={["Domains", "DNS Status", "Usable by", "Updated", ""]}>{rows}</StyledTable>

      <Flex justify="center" mt="md">
        <Stack>
          {filteredDomains.length > PER_PAGE && (
            <Pagination
              value={activePage}
              onChange={(p) => {
                navigate(routerState.name, {
                  ...routerState.params,
                  p: p.toString(),
                });
              }}
              total={totalPages}
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
