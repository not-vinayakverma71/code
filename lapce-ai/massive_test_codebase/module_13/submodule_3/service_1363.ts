// Service 1363
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1363 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}