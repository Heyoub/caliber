/**
 * MCP Index
 * Re-exports MCP client and types
 */

export { McpClient, McpError, createMcpClient } from './client';
export type { McpClientOptions, McpClientState } from './client';
export * from './types';
