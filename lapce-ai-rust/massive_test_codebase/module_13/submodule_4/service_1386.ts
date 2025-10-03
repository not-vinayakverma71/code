// Service 1386
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1386 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}