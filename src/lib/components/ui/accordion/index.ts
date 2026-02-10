/**
 * Accordion UI component – re-exports for shadcn-style accordion.
 * Added to support collapsible sections in HelpSidebar (e.g. "Lost access?").
 */
import Root from './accordion.svelte';
import Item from './accordion-item.svelte';
import Trigger from './accordion-trigger.svelte';
import Content from './accordion-content.svelte';

export { Root, Item, Trigger, Content };
