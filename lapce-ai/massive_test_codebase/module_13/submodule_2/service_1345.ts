// Service 1345
import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';

@Injectable({ providedIn: 'root' })
export class Service1345 {
async getData(): Promise<any> {
const response = await fetch('/api/data');
return response.json();
}
}