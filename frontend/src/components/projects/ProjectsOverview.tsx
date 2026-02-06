import { Flex, Pagination, Stack, Table, Text } from "@mantine/core";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useProjects } from "../../hooks/useProjects.ts";
import { IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewProject } from "./NewProject.tsx";
import { Link } from "../../Link.tsx";
import EditButton from "../EditButton.tsx";
import StyledTable from "../StyledTable.tsx";
import InfoAlert from "../InfoAlert.tsx";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";
import { useRemails } from "../../hooks/useRemails.ts";
import { useState } from "react";
import SearchInput from "../SearchInput.tsx";

const PER_PAGE = 20;
const SHOW_SEARCH = 10;

export default function ProjectsOverview() {
  const {
    state: { routerState },
    navigate,
  } = useRemails();
  const [opened, { open, close }] = useDisclosure(false);
  const { projects } = useProjects();
  const [searchQuery, setSearchQuery] = useState(routerState.params.q || "");

  if (projects === null) {
    return <Loader />;
  }

  const filteredProjects =
    searchQuery.length == 0
      ? projects
      : projects.filter((project) => project.name.toLowerCase().includes(searchQuery.toLowerCase()));

  const totalPages = Math.ceil(filteredProjects.length / PER_PAGE);
  const activePage = Math.min(Math.max(parseInt(routerState.params.p) || 1, 1), totalPages);

  const rows = filteredProjects.slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE).map((project) => (
    <Table.Tr key={project.id}>
      <Table.Td>
        <Link to="projects.project.emails" params={{ proj_id: project.id }}>
          {project.name}
        </Link>
      </Table.Td>
      <Table.Td>{formatDateTime(project.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <EditButton
          route="projects.project.settings"
          params={{
            proj_id: project.id,
          }}
        />
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <OrganizationHeader />

      <InfoAlert stateName="projects">
        Projects are used to group related work, such as different applications or environments.
      </InfoAlert>

      <NewProject opened={opened} close={close} />

      {(projects.length > SHOW_SEARCH || searchQuery.length > 0) && (
        <SearchInput searchQuery={searchQuery} setSearchQuery={setSearchQuery} />
      )}

      {searchQuery.length > 0 && filteredProjects.length == 0 && (
        <Text fs="italic" c="gray">
          No projects found...
        </Text>
      )}

      <StyledTable headers={["Name", "Updated", ""]}>{rows}</StyledTable>

      <Flex justify="center" mt="md">
        <Stack>
          {filteredProjects.length > PER_PAGE && (
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
            New Project
          </MaintainerButton>
        </Stack>
      </Flex>
    </>
  );
}
