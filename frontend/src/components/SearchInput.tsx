import { ActionIcon, TextInput } from "@mantine/core";
import { IconSearch, IconX } from "@tabler/icons-react";
import { useRemails } from "../hooks/useRemails";
import { Dispatch, SetStateAction } from "react";

interface Props {
  searchQuery: string;
  setSearchQuery: Dispatch<SetStateAction<string>>;
}

export default function SearchInput({ searchQuery, setSearchQuery }: Props) {
  const {
    state: { routerState },
    navigate,
  } = useRemails();

  return (
    <TextInput
      key={routerState.params.q}
      placeholder="Search..."
      leftSection={<IconSearch size={16} />}
      value={searchQuery}
      onChange={(ev) => setSearchQuery(ev.target.value)}
      onBlur={() => {
        navigate(routerState.name, {
          ...routerState.params,
          q: searchQuery,
        });
      }}
      rightSection={
        searchQuery ? (
          <ActionIcon
            variant="transparent"
            onClick={() => {
              navigate(routerState.name, {
                ...routerState.params,
                q: "",
              });
              setSearchQuery("");
            }}
          >
            <IconX size={16} />
          </ActionIcon>
        ) : null
      }
      mb="md"
    />
  );
}
