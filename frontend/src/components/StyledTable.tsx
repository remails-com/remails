import { Divider, Table, TableThProps } from "@mantine/core";
import React from "react";

interface StyledTableProps {
  headers: Array<TableThProps | string>;
  children?: React.ReactNode[];
}

export default function StyledTable({ headers, children }: StyledTableProps) {
  if (!children || children.length === 0) {
    return null;
  }

  return (
    <div>
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            {headers.map((props, i) =>
              typeof props === "string" ? <Table.Th key={i}>{props}</Table.Th> : <Table.Th key={i} {...props} />
            )}
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{children}</Table.Tbody>
      </Table>
      <Divider />
    </div>
  );
}
