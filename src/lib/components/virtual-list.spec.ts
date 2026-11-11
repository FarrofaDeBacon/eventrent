import { describe, expect, it } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";
import VirtualList from "./virtual-list.svelte";

interface BenchItem {
  id: string;
  name: string;
}

describe("VirtualList (Desempenho para 20.000 clientes)", () => {
  it("deve renderizar apenas uma janela limitada de nós DOM para 20.000 itens", async () => {
    const items: BenchItem[] = Array.from({ length: 20_000 }, (_, i) => ({
      id: `cust-${i}`,
      name: `Cliente Evento ${i}`,
    }));

    const children = createRawSnippet<[unknown, number]>((getItem, getIdx) => ({
      render: () => {
        const item = getItem() as BenchItem;
        return `<div data-testid="vrow-${getIdx()}">${item.name}</div>`;
      },
    }));

    const { container } = render(VirtualList, {
      props: {
        items,
        itemHeight: 72,
        height: 560,
        overscan: 5,
        getKey: (item: unknown) => (item as BenchItem).id,
        children,
      },
    });

    const renderedRows = container.querySelectorAll("[data-virtual-index]");
    // 560 / 72 = ~8 linhas visíveis + 2 * 5 overscan = ~18 nós DOM (muito menor que 20.000!)
    expect(renderedRows.length).toBeGreaterThan(0);
    expect(renderedRows.length).toBeLessThan(40);
    expect(container.textContent).toContain("Cliente Evento 0");
    expect(container.textContent).not.toContain("Cliente Evento 19999");

    // Simula scroll até o meio da lista (item ~10.000)
    const viewport = container.querySelector(
      '[data-testid="virtual-list-viewport"]',
    ) as HTMLDivElement;
    expect(viewport).not.toBeNull();
    viewport.scrollTop = 10_000 * 72;
    await fireEvent.scroll(viewport);

    const renderedAfterScroll = container.querySelectorAll(
      "[data-virtual-index]",
    );
    expect(renderedAfterScroll.length).toBeLessThan(40);
    expect(container.textContent).toContain("Cliente Evento 10000");
    expect(container.textContent).not.toContain("Cliente Evento 0");
  });
});
