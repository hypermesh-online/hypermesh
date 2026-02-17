import{r as n}from"./react-DlWwq_Dl.js";import{M as y,n as m,a as v,s as _}from"./web3-Bq1CduWo.js";var l={exports:{}},i={};/**
 * @license React
 * react-jsx-runtime.production.min.js
 *
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */var C=n,x=Symbol.for("react.element"),E=Symbol.for("react.fragment"),d=Object.prototype.hasOwnProperty,b=C.__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED.ReactCurrentOwner,O={key:!0,ref:!0,__self:!0,__source:!0};function f(t,r,a){var e,o={},s=null,u=null;a!==void 0&&(s=""+a),r.key!==void 0&&(s=""+r.key),r.ref!==void 0&&(u=r.ref);for(e in r)d.call(r,e)&&!O.hasOwnProperty(e)&&(o[e]=r[e]);if(t&&t.defaultProps)for(e in r=t.defaultProps,r)o[e]===void 0&&(o[e]=r[e]);return{$$typeof:x,type:t,key:s,ref:u,props:o,_owner:b.current}}i.Fragment=E;i.jsx=f;i.jsxs=f;l.exports=i;var h=l.exports,c=n.createContext(void 0),w=t=>{const r=n.useContext(c);if(!r)throw new Error("No QueryClient set, use QueryClientProvider to set one");return r},j=({client:t,children:r})=>(n.useEffect(()=>(t.mount(),()=>{t.unmount()}),[t]),h.jsx(c.Provider,{value:t,children:r}));function k(t,r){const a=w(),[e]=n.useState(()=>new y(a,t));n.useEffect(()=>{e.setOptions(t)},[e,t]);const o=n.useSyncExternalStore(n.useCallback(u=>e.subscribe(m.batchCalls(u)),[e]),()=>e.getCurrentResult(),()=>e.getCurrentResult()),s=n.useCallback((u,p)=>{e.mutate(u,p).catch(v)},[e]);if(o.error&&_(e.options.throwOnError,[o.error]))throw o.error;return{...o,mutate:s,mutateAsync:o.mutate}}export{j as Q,h as j,k as u};
