import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
export function cn(...values:ClassValue[]){return twMerge(clsx(values));}
export const money=(cents:number=0)=>new Intl.NumberFormat('es-MX',{style:'currency',currency:'MXN'}).format(cents/100);
export const quantity=(units:number=0)=>new Intl.NumberFormat('es-MX',{maximumFractionDigits:4}).format(units/10000);
export const units=['pza','m','kg','L','caja','rollo'];
export const today=()=>{const d=new Date();return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`};
export function parseScaled(value:string,scale=100):number{const n=Number(value);return Number.isFinite(n)?Math.round(n*scale):0;}
