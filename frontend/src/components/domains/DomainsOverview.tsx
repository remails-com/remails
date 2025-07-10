import { useRemails } from "../../hooks/useRemails.ts";
import { useDomains } from "../../hooks/useDomains.ts";
import { Loader } from "../../Loader.tsx";
import { Badge, Button, Flex, Table, Tooltip } from "@mantine/core";
import { formatDateTime } from "../../util.ts";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewDomain } from "./NewDomain.tsx";
import { useProjects } from "../../hooks/useProjects.ts";
import { Link } from "../../Link.tsx";
import { DomainVerificationResult } from "../../types.ts";

function formatVerificationStatus(status: DomainVerificationResult | null) {
  const errors = [];
  const warnings = [];

  if (!status) {
    return (
      <Badge color="gray" style={{ cursor: "pointer" }}>
        Unverified
      </Badge>
    );
  }

  for (const key of ["dkim", "spf", "dmarc", "a"] as const) {
    const value = status[key];
    if (value.status == "Error") {
      errors.push(key.toUpperCase());
    }
    if (value.status == "Warning") {
      warnings.push(key.toUpperCase());
    }
  }

  if (errors.length > 0) {
    let label = `${errors.join(", ")} record errors`;
    if (warnings.length > 0) {
      label += `, and ${warnings.join(", ")} record warnings`;
    }
    return (
      <Tooltip label={label}>
        <Badge style={{ cursor: "pointer" }}>Error</Badge>
      </Tooltip>
    );
  }

  if (warnings.length > 0) {
    return (
      <Tooltip label={`${warnings.join(", ")} record warnings`}>
        <Badge color="orange" style={{ cursor: "pointer" }}>
          Warning
        </Badge>
      </Tooltip>
    );
  }

  return (
    <Badge color="green" style={{ cursor: "pointer" }}>
      Verified
    </Badge>
  );
}

export default function DomainsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const {
    navigate,
    state: { routerState },
  } = useRemails();
  const { currentProject } = useProjects();
  const { domains } = useDomains();

  if (domains === null) {
    return <Loader />;
  }

  const route = routerState.params.proj_id ? "projects.project.domains.domain" : "domains.domain";

  const rows = domains.map((domain) => (
    <Table.Tr key={domain.id}>
      <Table.Td>
        <Link to={route} params={{ domain_id: domain.id }}>
          {domain.domain}
        </Link>
      </Table.Td>
      <Table.Td>
        <Link to={route} params={{ domain_id: domain.id }}>
          {formatVerificationStatus(domain.verification_status)}
        </Link>
      </Table.Td>
      <Table.Td>{formatDateTime(domain.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <Button
          variant="subtle"
          onClick={() => {
            navigate(route, {
              domain_id: domain.id,
            });
          }}
        >
          <IconEdit />
        </Button>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <NewDomain opened={opened} close={close} projectId={currentProject?.id || null} />
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Domain</Table.Th>
            <Table.Th>DNS Status</Table.Th>
            <Table.Th>Updated</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Domain
        </Button>
      </Flex>
    </>
  );
}
