// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

import AWS from 'aws-sdk';
import axios from 'axios';
import { config } from '@/config';
import { Logger } from '@/utils/Logger';
import {
  User,
  UserAccount,
  KYCValidationResult,
  KYCStatus,
  KYCLevel,
  DocumentType,
  DocumentStatus,
  KYCDocument,
  DocumentVerificationResult,
  RiskAssessment,
  RiskLevel,
  RiskFactor,
  SanctionsResult,
  PEPResult,
  TransactionLimits,
  GatewayError,
  ErrorCodes,
  PersonalInfo,
  Address,
} from '@/types';

export class KYCService {
  private logger: Logger;
  private s3: AWS.S3;
  private sanctionsAPI: SanctionsScreeningAPI;
  private jumioAPI: JumioAPI;

  constructor() {
    this.logger = new Logger('KYCService');
    
    // Initialize AWS S3 for document storage
    this.s3 = new AWS.S3({
      accessKeyId: config.aws.accessKeyId,
      secretAccessKey: config.aws.secretAccessKey,
      region: config.aws.region,
    });

    // Initialize API clients
    this.sanctionsAPI = new SanctionsScreeningAPI();
    this.jumioAPI = new JumioAPI();
  }

  /**
   * Perform comprehensive KYC validation for a user
   */
  async performKYCValidation(user: UserAccount): Promise<KYCValidationResult> {
    try {
      this.logger.info('Starting KYC validation', { userId: user.id });

      // Step 1: Document verification
      const documentResults = await this.verifyDocuments(user.documents);
      
      // Step 2: Identity verification
      const identityResult = await this.verifyIdentity(user.personalInfo, documentResults);
      
      // Step 3: Risk assessment
      const riskAssessment = await this.performRiskAssessment(user);
      
      // Step 4: Sanctions screening
      const sanctionsResult = await this.screenSanctions(user.personalInfo);
      
      // Step 5: PEP screening
      const pepResult = await this.screenPEP(user.personalInfo);
      
      // Step 6: Determine KYC level and status
      const { status, level, rejectionReason } = this.determineKYCLevel(
        documentResults,
        identityResult,
        riskAssessment,
        sanctionsResult,
        pepResult
      );

      // Step 7: Set transaction limits
      const approvedLimits = this.getTransactionLimits(level, riskAssessment.level);

      // Step 8: Identify required documents
      const requiredDocuments = this.getRequiredDocuments(level, documentResults);

      const result: KYCValidationResult = {
        userId: user.id,
        status,
        level,
        approvedLimits,
        rejectionReason,
        requiredDocuments,
        completedAt: new Date(),
      };

      // Store validation result
      await this.storeKYCResult(result, riskAssessment, sanctionsResult, pepResult);

      this.logger.info('KYC validation completed', { 
        userId: user.id, 
        status, 
        level,
        riskLevel: riskAssessment.level 
      });

      return result;

    } catch (error) {
      this.logger.error('KYC validation failed', { error, userId: user.id });
      throw new GatewayError(
        ErrorCodes.SYSTEM_ERROR,
        'KYC validation failed',
        500,
        error
      );
    }
  }

  /**
   * Upload and process KYC documents
   */
  async uploadDocument(params: {
    userId: string;
    documentType: DocumentType;
    fileBuffer: Buffer;
    fileName: string;
    mimeType: string;
  }): Promise<KYCDocument> {
    try {
      this.logger.info('Uploading KYC document', {
        userId: params.userId,
        documentType: params.documentType,
        fileName: params.fileName
      });

      // Validate file
      this.validateDocumentFile(params.fileBuffer, params.mimeType);

      // Generate S3 key
      const s3Key = this.generateS3Key(params.userId, params.documentType, params.fileName);

      // Upload to S3
      await this.s3.upload({
        Bucket: config.aws.s3.bucketName,
        Key: s3Key,
        Body: params.fileBuffer,
        ContentType: params.mimeType,
        ServerSideEncryption: 'AES256',
        Metadata: {
          userId: params.userId,
          documentType: params.documentType,
          uploadedBy: 'kyc-service',
        },
      }).promise();

      // Create document record
      const document: KYCDocument = {
        id: this.generateDocumentId(),
        type: params.documentType,
        status: DocumentStatus.UPLOADED,
        s3Key,
        uploadedAt: new Date(),
        expiresAt: this.calculateDocumentExpiry(params.documentType),
      };

      // Store document metadata in database
      await this.storeDocumentMetadata(params.userId, document);

      // Trigger async document verification
      await this.triggerDocumentVerification(params.userId, document);

      this.logger.info('Document uploaded successfully', {
        userId: params.userId,
        documentId: document.id,
        s3Key
      });

      return document;

    } catch (error) {
      this.logger.error('Failed to upload document', { error, userId: params.userId });
      throw new GatewayError(
        ErrorCodes.INVALID_REQUEST,
        'Document upload failed',
        400,
        error
      );
    }
  }

  /**
   * Verify uploaded documents using Jumio
   */
  private async verifyDocuments(documents: KYCDocument[]): Promise<Map<DocumentType, DocumentVerificationResult>> {
    const results = new Map<DocumentType, DocumentVerificationResult>();

    for (const document of documents) {
      try {
        if (document.status === DocumentStatus.UPLOADED) {
          const result = await this.jumioAPI.verifyDocument(document);
          results.set(document.type, result);
          
          // Update document status
          document.status = result.isValid ? DocumentStatus.VERIFIED : DocumentStatus.REJECTED;
          document.verificationResult = result;
        } else if (document.verificationResult) {
          results.set(document.type, document.verificationResult);
        }
      } catch (error) {
        this.logger.error('Document verification failed', { error, documentId: document.id });
        results.set(document.type, {
          isValid: false,
          confidence: 0,
          extractedData: {},
          flags: ['VERIFICATION_FAILED'],
          provider: 'jumio',
          timestamp: new Date(),
        });
      }
    }

    return results;
  }

  /**
   * Verify identity consistency across documents
   */
  private async verifyIdentity(
    personalInfo: PersonalInfo, 
    documentResults: Map<DocumentType, DocumentVerificationResult>
  ): Promise<{
    isValid: boolean;
    confidence: number;
    discrepancies: string[];
  }> {
    const discrepancies: string[] = [];
    let totalConfidence = 0;
    let validDocuments = 0;

    // Check government ID
    const govIdResult = documentResults.get(DocumentType.GOVERNMENT_ID) || 
                       documentResults.get(DocumentType.PASSPORT) ||
                       documentResults.get(DocumentType.DRIVERS_LICENSE);

    if (govIdResult) {
      validDocuments++;
      totalConfidence += govIdResult.confidence;

      // Verify name match
      const extractedName = `${govIdResult.extractedData.firstName} ${govIdResult.extractedData.lastName}`;
      const providedName = `${personalInfo.firstName} ${personalInfo.lastName}`;
      
      if (!this.namesMatch(extractedName, providedName)) {
        discrepancies.push('Name mismatch between document and provided information');
      }

      // Verify date of birth
      if (govIdResult.extractedData.dateOfBirth && 
          govIdResult.extractedData.dateOfBirth !== personalInfo.dateOfBirth) {
        discrepancies.push('Date of birth mismatch');
      }
    }

    // Check proof of address
    const addressResult = documentResults.get(DocumentType.PROOF_OF_ADDRESS);
    if (addressResult) {
      validDocuments++;
      totalConfidence += addressResult.confidence;

      // Verify address match
      if (!this.addressMatches(addressResult.extractedData.address, personalInfo.address)) {
        discrepancies.push('Address mismatch between document and provided information');
      }
    }

    const averageConfidence = validDocuments > 0 ? totalConfidence / validDocuments : 0;
    const isValid = discrepancies.length === 0 && averageConfidence > 0.8;

    return {
      isValid,
      confidence: averageConfidence,
      discrepancies,
    };
  }

  /**
   * Perform comprehensive risk assessment
   */
  private async performRiskAssessment(user: UserAccount): Promise<RiskAssessment> {
    const riskFactors: RiskFactor[] = [];
    let totalScore = 0;

    // Geographic risk
    const countryRisk = this.assessCountryRisk(user.personalInfo.nationality);
    if (countryRisk.impact > 0) {
      riskFactors.push(countryRisk);
      totalScore += countryRisk.impact;
    }

    // Address risk
    const addressRisk = this.assessAddressRisk(user.personalInfo.address);
    if (addressRisk.impact > 0) {
      riskFactors.push(addressRisk);
      totalScore += addressRisk.impact;
    }

    // Age risk
    const ageRisk = this.assessAgeRisk(user.personalInfo.dateOfBirth);
    if (ageRisk.impact > 0) {
      riskFactors.push(ageRisk);
      totalScore += ageRisk.impact;
    }

    // Behavioral risk (simplified - would include transaction patterns)
    const behavioralRisk = await this.assessBehavioralRisk(user.id);
    if (behavioralRisk.impact > 0) {
      riskFactors.push(behavioralRisk);
      totalScore += behavioralRisk.impact;
    }

    // Determine risk level
    let level: RiskLevel;
    if (totalScore >= 75) level = RiskLevel.PROHIBITED;
    else if (totalScore >= 50) level = RiskLevel.HIGH;
    else if (totalScore >= 25) level = RiskLevel.MEDIUM;
    else level = RiskLevel.LOW;

    // Perform sanctions screening
    const sanctions = await this.screenSanctions(user.personalInfo);
    
    // Perform PEP screening
    const pep = await this.screenPEP(user.personalInfo);

    return {
      score: totalScore,
      level,
      factors: riskFactors,
      sanctions,
      pep,
      lastAssessed: new Date(),
    };
  }

  /**
   * Screen against sanctions lists
   */
  private async screenSanctions(personalInfo: PersonalInfo): Promise<SanctionsResult> {
    try {
      const result = await this.sanctionsAPI.screen({
        firstName: personalInfo.firstName,
        lastName: personalInfo.lastName,
        dateOfBirth: personalInfo.dateOfBirth,
        nationality: personalInfo.nationality,
        address: personalInfo.address,
      });

      return result;

    } catch (error) {
      this.logger.error('Sanctions screening failed', { error });
      return {
        isMatch: false,
        lists: [],
        confidence: 0,
        details: 'Screening failed',
      };
    }
  }

  /**
   * Screen for Politically Exposed Persons (PEP)
   */
  private async screenPEP(personalInfo: PersonalInfo): Promise<PEPResult> {
    try {
      const result = await this.sanctionsAPI.screenPEP({
        firstName: personalInfo.firstName,
        lastName: personalInfo.lastName,
        nationality: personalInfo.nationality,
      });

      return result;

    } catch (error) {
      this.logger.error('PEP screening failed', { error });
      return {
        isPEP: false,
        details: 'Screening failed',
      };
    }
  }

  /**
   * Determine KYC level based on verification results
   */
  private determineKYCLevel(
    documentResults: Map<DocumentType, DocumentVerificationResult>,
    identityResult: { isValid: boolean; confidence: number; discrepancies: string[] },
    riskAssessment: RiskAssessment,
    sanctionsResult: SanctionsResult,
    pepResult: PEPResult
  ): {
    status: KYCStatus;
    level: KYCLevel;
    rejectionReason?: string;
  } {
    // Immediate rejection criteria
    if (sanctionsResult.isMatch) {
      return {
        status: KYCStatus.REJECTED,
        level: KYCLevel.LEVEL_0,
        rejectionReason: 'Sanctions list match',
      };
    }

    if (riskAssessment.level === RiskLevel.PROHIBITED) {
      return {
        status: KYCStatus.REJECTED,
        level: KYCLevel.LEVEL_0,
        rejectionReason: 'Risk assessment: Prohibited risk level',
      };
    }

    if (!identityResult.isValid) {
      return {
        status: KYCStatus.REJECTED,
        level: KYCLevel.LEVEL_0,
        rejectionReason: `Identity verification failed: ${identityResult.discrepancies.join(', ')}`,
      };
    }

    // Determine level based on documents and risk
    let level = KYCLevel.LEVEL_0;
    
    // Level 1: Basic ID verification
    const hasValidId = documentResults.has(DocumentType.GOVERNMENT_ID) ||
                      documentResults.has(DocumentType.PASSPORT) ||
                      documentResults.has(DocumentType.DRIVERS_LICENSE);
    
    if (hasValidId && identityResult.confidence > 0.8) {
      level = KYCLevel.LEVEL_1;
    }

    // Level 2: Enhanced verification with address proof
    const hasAddressProof = documentResults.has(DocumentType.PROOF_OF_ADDRESS);
    if (level >= KYCLevel.LEVEL_1 && hasAddressProof && identityResult.confidence > 0.9) {
      level = KYCLevel.LEVEL_2;
    }

    // Level 3: Full verification with additional checks
    const hasSelfie = documentResults.has(DocumentType.SELFIE);
    const hasFinancialDoc = documentResults.has(DocumentType.BANK_STATEMENT);
    
    if (level >= KYCLevel.LEVEL_2 && 
        hasSelfie && 
        hasFinancialDoc && 
        riskAssessment.level <= RiskLevel.MEDIUM &&
        identityResult.confidence > 0.95) {
      level = KYCLevel.LEVEL_3;
    }

    // Handle PEP status
    let status = KYCStatus.APPROVED;
    if (pepResult.isPEP || riskAssessment.level === RiskLevel.HIGH) {
      status = KYCStatus.UNDER_REVIEW;
    }

    return { status, level };
  }

  /**
   * Get transaction limits based on KYC level and risk
   */
  private getTransactionLimits(kycLevel: KYCLevel, riskLevel: RiskLevel): TransactionLimits {
    const baseLimits = config.limits.kycLimits[`level${kycLevel}`];
    
    // Apply risk-based multipliers
    let multiplier = 1.0;
    switch (riskLevel) {
      case RiskLevel.HIGH:
        multiplier = 0.5;
        break;
      case RiskLevel.MEDIUM:
        multiplier = 0.8;
        break;
      case RiskLevel.LOW:
        multiplier = 1.0;
        break;
      case RiskLevel.PROHIBITED:
        multiplier = 0;
        break;
    }

    return {
      dailyDeposit: Math.floor(baseLimits.daily * multiplier),
      dailyWithdrawal: Math.floor(baseLimits.daily * multiplier * 0.8), // Withdrawals more restrictive
      monthlyDeposit: Math.floor(baseLimits.monthly * multiplier),
      monthlyWithdrawal: Math.floor(baseLimits.monthly * multiplier * 0.8),
      singleTransactionMax: Math.floor(baseLimits.single * multiplier),
      currency: 'USD',
    };
  }

  /**
   * Get required documents for KYC level
   */
  private getRequiredDocuments(
    targetLevel: KYCLevel, 
    documentResults: Map<DocumentType, DocumentVerificationResult>
  ): DocumentType[] {
    const required: DocumentType[] = [];
    const verified = Array.from(documentResults.keys()).filter(
      type => documentResults.get(type)?.isValid
    );

    switch (targetLevel) {
      case KYCLevel.LEVEL_3:
        if (!verified.includes(DocumentType.BANK_STATEMENT)) {
          required.push(DocumentType.BANK_STATEMENT);
        }
        if (!verified.includes(DocumentType.SELFIE)) {
          required.push(DocumentType.SELFIE);
        }
        // Fall through
      
      case KYCLevel.LEVEL_2:
        if (!verified.includes(DocumentType.PROOF_OF_ADDRESS)) {
          required.push(DocumentType.PROOF_OF_ADDRESS);
        }
        // Fall through
      
      case KYCLevel.LEVEL_1:
        const hasValidId = verified.some(type => 
          [DocumentType.GOVERNMENT_ID, DocumentType.PASSPORT, DocumentType.DRIVERS_LICENSE].includes(type)
        );
        if (!hasValidId) {
          required.push(DocumentType.GOVERNMENT_ID);
        }
        break;
    }

    return required;
  }

  /**
   * Risk assessment helper methods
   */
  private assessCountryRisk(nationality: string): RiskFactor {
    // High-risk countries (simplified list)
    const highRiskCountries = ['AF', 'IR', 'KP', 'SY', 'YE'];
    const mediumRiskCountries = ['BD', 'BO', 'GH', 'LA', 'LK', 'MG', 'MN', 'MZ', 'NP', 'PK', 'TZ', 'UG'];

    if (highRiskCountries.includes(nationality.toUpperCase())) {
      return {
        type: 'GEOGRAPHIC_RISK',
        description: 'High-risk jurisdiction',
        impact: 40,
        severity: 'HIGH',
      };
    }

    if (mediumRiskCountries.includes(nationality.toUpperCase())) {
      return {
        type: 'GEOGRAPHIC_RISK',
        description: 'Medium-risk jurisdiction',
        impact: 20,
        severity: 'MEDIUM',
      };
    }

    return {
      type: 'GEOGRAPHIC_RISK',
      description: 'Low-risk jurisdiction',
      impact: 0,
      severity: 'LOW',
    };
  }

  private assessAddressRisk(address: Address): RiskFactor {
    // Simplified address risk assessment
    // In practice, would check against known high-risk areas
    return {
      type: 'ADDRESS_RISK',
      description: 'Address verification',
      impact: 0,
      severity: 'LOW',
    };
  }

  private assessAgeRisk(dateOfBirth: string): RiskFactor {
    const age = this.calculateAge(dateOfBirth);
    
    if (age < 18) {
      return {
        type: 'AGE_RISK',
        description: 'Under 18 years old',
        impact: 100, // Prohibited
        severity: 'HIGH',
      };
    }

    if (age < 21) {
      return {
        type: 'AGE_RISK',
        description: 'Young adult (under 21)',
        impact: 10,
        severity: 'MEDIUM',
      };
    }

    return {
      type: 'AGE_RISK',
      description: 'Appropriate age',
      impact: 0,
      severity: 'LOW',
    };
  }

  private async assessBehavioralRisk(userId: string): Promise<RiskFactor> {
    // This would analyze user behavior patterns
    // Simplified for demonstration
    return {
      type: 'BEHAVIORAL_RISK',
      description: 'No concerning behavioral patterns',
      impact: 0,
      severity: 'LOW',
    };
  }

  /**
   * Utility methods
   */
  private validateDocumentFile(fileBuffer: Buffer, mimeType: string): void {
    const maxSize = 10 * 1024 * 1024; // 10MB
    const allowedTypes = ['image/jpeg', 'image/png', 'application/pdf'];

    if (fileBuffer.length > maxSize) {
      throw new GatewayError(
        ErrorCodes.INVALID_REQUEST,
        'File size too large (max 10MB)',
        400
      );
    }

    if (!allowedTypes.includes(mimeType)) {
      throw new GatewayError(
        ErrorCodes.INVALID_REQUEST,
        'Invalid file type. Only JPEG, PNG, and PDF files are allowed',
        400
      );
    }
  }

  private generateS3Key(userId: string, documentType: DocumentType, fileName: string): string {
    const timestamp = Date.now();
    const sanitizedFileName = fileName.replace(/[^a-zA-Z0-9.-]/g, '_');
    return `kyc-documents/${userId}/${documentType}/${timestamp}_${sanitizedFileName}`;
  }

  private generateDocumentId(): string {
    return `doc_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private calculateDocumentExpiry(documentType: DocumentType): Date | undefined {
    // Government IDs typically expire, address proofs should be recent
    const expiryMonths: Record<DocumentType, number | null> = {
      [DocumentType.GOVERNMENT_ID]: null, // Use document's actual expiry
      [DocumentType.PASSPORT]: null, // Use document's actual expiry
      [DocumentType.DRIVERS_LICENSE]: null, // Use document's actual expiry
      [DocumentType.PROOF_OF_ADDRESS]: 3, // 3 months
      [DocumentType.BANK_STATEMENT]: 3, // 3 months
      [DocumentType.SELFIE]: 12, // 1 year
    };

    const months = expiryMonths[documentType];
    if (months) {
      const expiry = new Date();
      expiry.setMonth(expiry.getMonth() + months);
      return expiry;
    }

    return undefined;
  }

  private namesMatch(extracted: string, provided: string): boolean {
    // Fuzzy name matching logic
    const normalize = (name: string) => name.toLowerCase().replace(/[^a-z]/g, '');
    return normalize(extracted) === normalize(provided);
  }

  private addressMatches(extracted: any, provided: Address): boolean {
    // Simplified address matching
    // In practice, would use more sophisticated matching
    return true; // Placeholder
  }

  private calculateAge(dateOfBirth: string): number {
    const dob = new Date(dateOfBirth);
    const now = new Date();
    let age = now.getFullYear() - dob.getFullYear();
    const monthDiff = now.getMonth() - dob.getMonth();
    
    if (monthDiff < 0 || (monthDiff === 0 && now.getDate() < dob.getDate())) {
      age--;
    }
    
    return age;
  }

  private async storeKYCResult(
    result: KYCValidationResult,
    riskAssessment: RiskAssessment,
    sanctionsResult: SanctionsResult,
    pepResult: PEPResult
  ): Promise<void> {
    // Store in database - implementation depends on chosen database
    this.logger.info('Storing KYC result', { userId: result.userId });
  }

  private async storeDocumentMetadata(userId: string, document: KYCDocument): Promise<void> {
    // Store document metadata in database
    this.logger.info('Storing document metadata', { userId, documentId: document.id });
  }

  private async triggerDocumentVerification(userId: string, document: KYCDocument): Promise<void> {
    // Trigger async document verification job
    this.logger.info('Triggering document verification', { userId, documentId: document.id });
  }
}

/**
 * Sanctions Screening API Client
 */
class SanctionsScreeningAPI {
  private logger: Logger;

  constructor() {
    this.logger = new Logger('SanctionsAPI');
  }

  async screen(params: {
    firstName: string;
    lastName: string;
    dateOfBirth: string;
    nationality: string;
    address: Address;
  }): Promise<SanctionsResult> {
    try {
      // Implementation would call actual sanctions screening service
      // This is a simplified mock
      return {
        isMatch: false,
        lists: [],
        confidence: 0.95,
      };
    } catch (error) {
      this.logger.error('Sanctions screening failed', { error });
      throw error;
    }
  }

  async screenPEP(params: {
    firstName: string;
    lastName: string;
    nationality: string;
  }): Promise<PEPResult> {
    try {
      // Implementation would call actual PEP screening service
      return {
        isPEP: false,
      };
    } catch (error) {
      this.logger.error('PEP screening failed', { error });
      throw error;
    }
  }
}

/**
 * Jumio API Client
 */
class JumioAPI {
  private logger: Logger;
  private baseUrl: string;
  private token: string;
  private secret: string;

  constructor() {
    this.logger = new Logger('JumioAPI');
    this.baseUrl = 'https://netverify.com/api/netverify/v2';
    this.token = config.kyc.jumio.apiToken;
    this.secret = config.kyc.jumio.apiSecret;
  }

  async verifyDocument(document: KYCDocument): Promise<DocumentVerificationResult> {
    try {
      // Implementation would call actual Jumio API
      // This is a simplified mock that returns success for demonstration
      return {
        isValid: true,
        confidence: 0.95,
        extractedData: {
          firstName: 'John',
          lastName: 'Doe',
          dateOfBirth: '1990-01-01',
          documentNumber: '123456789',
          address: {
            street: '123 Main St',
            city: 'Anytown',
            state: 'NY',
            postalCode: '12345',
            country: 'US'
          }
        },
        flags: [],
        provider: 'jumio',
        timestamp: new Date(),
      };
    } catch (error) {
      this.logger.error('Jumio verification failed', { error });
      throw error;
    }
  }
}