// Service 208
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service208 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}