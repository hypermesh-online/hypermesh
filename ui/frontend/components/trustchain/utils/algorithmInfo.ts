// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

export const algorithmInfo: Record<string, { name: string; type: string; security: string; keySize: string; description: string; color: string; bgColor: string }> = {
  'FALCON-1024': {
    name: 'FALCON-1024',
    type: 'Post-Quantum Digital Signature',
    security: 'NIST Level 5',
    keySize: '1024 bits',
    description: 'Quantum-resistant signature algorithm based on lattice cryptography',
    color: 'text-purple-400',
    bgColor: 'bg-purple-500/10'
  },
  'Kyber-768': {
    name: 'Kyber-768',
    type: 'Post-Quantum Key Encapsulation',
    security: 'NIST Level 3',
    keySize: '768 bits',
    description: 'Quantum-resistant key exchange mechanism',
    color: 'text-blue-400',
    bgColor: 'bg-blue-500/10'
  },
  'RSA-2048': {
    name: 'RSA-2048',
    type: 'Classical Public Key',
    security: 'Legacy',
    keySize: '2048 bits',
    description: 'Traditional RSA encryption (not quantum-resistant)',
    color: 'text-yellow-400',
    bgColor: 'bg-yellow-500/10'
  }
};

export const commonExtensions: Record<string, string> = {
  '2.5.29.15': 'Key Usage',
  '2.5.29.37': 'Extended Key Usage',
  '2.5.29.17': 'Subject Alternative Name',
  '2.5.29.18': 'Issuer Alternative Name',
  '2.5.29.31': 'CRL Distribution Points',
  '1.3.6.1.5.5.7.1.1': 'Authority Information Access',
  '2.5.29.19': 'Basic Constraints',
  '2.5.29.14': 'Subject Key Identifier',
  '2.5.29.35': 'Authority Key Identifier',
  '2.5.29.32': 'Certificate Policies'
};