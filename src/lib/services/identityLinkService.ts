/**
 * Thin invoke wrappers for identity discovery/link/detail commands.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  IdentityDetails,
  LinkableIdentity,
  LinkedIdentity,
  LinkIdentityRequest,
  SetLinkedIdentityFavoriteRequest,
  UnlinkIdentityRequest
} from '$lib/types/wallet.js';

export async function discoverLinkableIdentities(): Promise<LinkableIdentity[]> {
  return invoke<LinkableIdentity[]>('discover_linkable_identities');
}

export async function getLinkedIdentities(): Promise<LinkedIdentity[]> {
  return invoke<LinkedIdentity[]>('get_linked_identities');
}

export async function linkIdentity(request: LinkIdentityRequest): Promise<LinkedIdentity[]> {
  return invoke<LinkedIdentity[]>('link_identity', {
    request: {
      identityAddress: request.identityAddress
    }
  });
}

export async function unlinkIdentity(request: UnlinkIdentityRequest): Promise<LinkedIdentity[]> {
  return invoke<LinkedIdentity[]>('unlink_identity', {
    request: {
      identityAddress: request.identityAddress
    }
  });
}

export async function getIdentityDetails(identityAddress: string): Promise<IdentityDetails> {
  return invoke<IdentityDetails>('get_identity_details', {
    identity_address: identityAddress
  });
}

export async function setLinkedIdentityFavorite(
  request: SetLinkedIdentityFavoriteRequest
): Promise<LinkedIdentity[]> {
  return invoke<LinkedIdentity[]>('set_linked_identity_favorite', {
    request: {
      identityAddress: request.identityAddress,
      favorite: request.favorite
    }
  });
}
